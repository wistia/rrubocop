//! Find eager relative dates without removing observable constant APIs.
use crate::cop::shared::{argument_nodes, call_method_name, named_kids, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct RelativeDateConstant;
fn constant(node: Node<'_>) -> bool {
    matches!(node.kind(), "constant" | "scope_resolution")
}
fn relative(source: &SourceFile, node: Node<'_>) -> Option<String> {
    let name = if node.kind() == "identifier" {
        crate::cop::ported::bare_call_identifier(source, node).then(|| node_text(source, node))?
    } else if node.kind() == "call"
        && !crate::cop::shared::is_setter_call(node)
        && node
            .child_by_field_name("operator")
            .is_none_or(|n| node_text(source, n) != "&.")
        && argument_nodes(node).is_empty()
    {
        String::from_utf8_lossy(call_method_name(source, node)?).into_owned()
    } else {
        return None;
    };
    matches!(
        name.as_str(),
        "since" | "from_now" | "after" | "ago" | "until" | "before" | "yesterday" | "tomorrow"
    )
    .then_some(name)
}
fn nested(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> Option<String> {
    let block = if node.kind() == "lambda" {
        Some(node)
    } else {
        node.child_by_field_name("block")
    };
    if block.is_some_and(|block| {
        config.get_f64("TargetRubyVersion", 3.4) < 2.7
            || crate::cop::ported::numbered_bindings(source, block).is_none()
    }) {
        return None;
    }
    let children = if node.kind() == "heredoc_beginning" {
        crate::cop::shared::heredoc_interpolations(source, node).unwrap_or_default()
    } else {
        named_kids(node)
    };
    for child in children {
        if matches!(child.kind(), "comment" | "identifier")
            && node.child_by_field_name("method") == Some(child)
        {
            continue;
        }
        if let Some(name) = nested(source, child, config) {
            return Some(name);
        }
    }
    relative(source, node)
}
#[derive(Clone, Copy)]
enum DateValue {
    Number,
    Duration,
    DateClass,
    TimeClass,
    Date,
    Time,
}
fn passive(source: &SourceFile, node: Node<'_>) -> Option<DateValue> {
    if matches!(node.kind(), "integer" | "float") {
        return node_text(source, node)
            .replace('_', "")
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && value.abs() <= 10_000.0)
            .map(|_| DateValue::Number);
    }
    if matches!(node.kind(), "constant" | "scope_resolution") {
        return match node_text(source, node).as_str() {
            "Date" | "::Date" | "DateTime" | "::DateTime" => Some(DateValue::DateClass),
            "Time" | "::Time" => Some(DateValue::TimeClass),
            _ => None,
        };
    }
    if node.kind() != "call"
        || !argument_nodes(node).is_empty()
        || node.child_by_field_name("block").is_some()
    {
        return None;
    }
    let receiver = passive(source, node.child_by_field_name("receiver")?)?;
    let name = String::from_utf8_lossy(call_method_name(source, node)?);
    match (receiver, name.as_ref()) {
        (
            DateValue::Number,
            "second" | "seconds" | "minute" | "minutes" | "hour" | "hours" | "day" | "days"
            | "week" | "weeks" | "month" | "months" | "year" | "years",
        ) => Some(DateValue::Duration),
        (DateValue::Duration, "ago" | "since" | "from_now" | "after" | "until" | "before") => {
            Some(DateValue::Time)
        }
        (DateValue::DateClass, "today" | "current" | "yesterday" | "tomorrow") => {
            Some(DateValue::Date)
        }
        (DateValue::TimeClass, "now" | "current") => Some(DateValue::Time),
        (DateValue::Date, "yesterday" | "tomorrow" | "to_date") => Some(DateValue::Date),
        (DateValue::Time, "yesterday" | "tomorrow") => Some(DateValue::Time),
        (DateValue::Time, "to_date") => Some(DateValue::Date),
        _ => None,
    }
}
const BASE_METHODS: &str = include_str!("relative_date_base_methods.txt");
fn replacement(
    source: &SourceFile,
    node: Node<'_>,
    left: Node<'_>,
    right: Node<'_>,
) -> Option<String> {
    if left.kind() != "constant"
        || passive(source, right).is_none()
        || !node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "program" | "body_statement" | "block_body"))
    {
        return None;
    }
    let name = node_text(source, left);
    let method: String = name.chars().flat_map(char::to_lowercase).collect();
    if BASE_METHODS
        .split_ascii_whitespace()
        .any(|name| name == method)
    {
        return None;
    }
    let mut count = 0;
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let mut stack = vec![root];
    while let Some(child) = stack.pop() {
        if child.kind() == "superclass" {
            return None;
        }
        if child.kind() == "constant" && node_text(source, child) == name {
            count += 1;
        }
        if matches!(
            child.kind(),
            "method" | "singleton_method" | "comment" | "heredoc_beginning"
        ) {
            return None;
        }
        if child.kind() == "identifier"
            && matches!(
                node_text(source, child).as_str(),
                "__LINE__"
                    | "eval"
                    | "class_eval"
                    | "module_eval"
                    | "const_get"
                    | "const_defined?"
                    | "constants"
                    | "remove_const"
                    | "private_constant"
                    | "public_constant"
                    | "autoload"
                    | "deprecate_constant"
            )
        {
            return None;
        }
        stack.extend(named_kids(child));
    }
    if count != 1 {
        return None;
    }
    let (_, column) = source.offset_to_line_col(node.start_byte());
    let indent = " ".repeat(column);
    Some(format!(
        "def self.{method}\n{indent}{indent}{}\n{indent}end",
        node_text(source, right)
    ))
}
impl Cop for RelativeDateConstant {
    fn name(&self) -> &'static str {
        "Rails/RelativeDateConstant"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["assignment", "operator_assignment"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        let Some(right) = node.child_by_field_name("right") else {
            return;
        };
        let mut offenses = Vec::new();
        if node.kind() == "assignment" && constant(left) {
            if let Some(name) = nested(source, node, config) {
                offenses.push((node.start_byte(), node.end_byte(), name));
            }
        } else if node.kind() == "operator_assignment"
            && constant(left)
            && right.child_by_field_name("block").is_none()
            && node
                .child_by_field_name("operator")
                .is_some_and(|n| node_text(source, n) == "||=")
        {
            if let Some(name) = relative(source, right) {
                offenses.push((node.start_byte(), node.end_byte(), name));
            }
        } else if node.kind() == "assignment"
            && left.kind() == "left_assignment_list"
            && matches!(right.kind(), "array" | "right_assignment_list")
        {
            for (name, value) in named_kids(left)
                .into_iter()
                .filter(|n| n.kind() != "comment")
                .zip(
                    named_kids(right)
                        .into_iter()
                        .filter(|n| n.kind() != "comment"),
                )
            {
                if constant(name) {
                    if let Some(method) = nested(source, value, config) {
                        offenses.push((name.start_byte(), value.end_byte(), method));
                    }
                }
            }
        }
        for (start, end, name) in offenses {
            let (line, col) = source.offset_to_line_col(start);
            let mut d = self.diagnostic(
                source,
                line,
                col,
                format!("Do not assign `{name}` to constants as it will be evaluated only once."),
            );
            d.highlight_length = super::blank::highlight_length(source, start, end);
            d.correctable = false;
            if node.kind() == "assignment" && constant(left) {
                if let Some(text) = replacement(source, node, left, right) {
                    let mut proposed = source.content.clone();
                    proposed.splice(node.start_byte()..node.end_byte(), text.bytes());
                    let proposed = SourceFile::from_bytes("date.rb", proposed);
                    if !crate::parse::parse_ruby(&proposed)
                        .is_ok_and(|tree| !tree.root_node().has_error())
                    {
                        diagnostics.push(d);
                        continue;
                    }
                    d.correctable = true;
                    d.corrected = crate::cop::shared::push_replace(
                        &mut corrections,
                        node.start_byte(),
                        node.end_byte(),
                        text,
                        self.name(),
                    );
                }
            }
            diagnostics.push(d);
        }
    }
}
