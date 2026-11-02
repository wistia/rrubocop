//! Remove redundant validation nil flags without dropping dynamic flag evaluation.
use crate::cop::shared::ruby_expression::{
    ShorthandValue, float_value, integer_ast, plain_string, plain_symbol, regular_send_receiver,
    shorthand_value,
};
use crate::cop::shared::{
    call_method_name, collect_comments, named_kids, node_bytes, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct RedundantAllowNil;
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect()
}
fn flags<'a>(node: Node<'a>, root: bool, source: &SourceFile) -> Option<(Node<'a>, Node<'a>)> {
    let mut nil = None;
    let mut blank = None;
    for child in children(node) {
        if root && node.child_by_field_name("block") == Some(child) {
            continue;
        }
        if child.kind() == "pair" {
            if let Some(key) = child.child_by_field_name("key") {
                match node_text(source, key).as_str() {
                    "allow_nil" => nil = Some(child),
                    "allow_blank" => blank = Some(child),
                    _ => {}
                }
            }
        }
        if let Some(pair) = nil.zip(blank) {
            return Some(pair);
        }
        if let Some(pair) = flags(child, false, source) {
            return Some(pair);
        }
    }
    None
}
fn value(pair: Node<'_>) -> Option<Node<'_>> {
    pair.child_by_field_name("value")
        .or_else(|| pair.child_by_field_name("key"))
}
fn value_type<'a>(source: &SourceFile, node: Node<'a>, config: &CopConfig) -> &'a str {
    if let Some(role) = shorthand_value(source, node) {
        return match role {
            ShorthandValue::Constant => "const",
            ShorthandValue::Local => "lvar",
            ShorthandValue::Send => "send",
        };
    }
    if integer_ast(source, node).is_some() {
        return "int";
    }
    if float_value(source, node).is_some() {
        return "float";
    }
    if matches!(node.kind(), "call" | "lambda")
        && (node.kind() == "lambda" || node.child_by_field_name("block").is_some())
    {
        let block = if node.kind() == "lambda" {
            Some(node)
        } else {
            node.child_by_field_name("block")
        };
        return if config.get_f64("TargetRubyVersion", 3.4) >= 2.7
            && block.is_some_and(|n| crate::cop::ported::numbered_bindings(source, n).is_some())
        {
            "numblock"
        } else {
            "block"
        };
    }
    if regular_send_receiver(source, node).is_some() {
        return "send";
    }
    if node.kind() == "delimited_symbol" {
        return if plain_symbol(source, node).is_some() {
            "sym"
        } else {
            "dsym"
        };
    }
    if crate::cop::ported::static_symbol(source, node).is_some() {
        return if plain_symbol(source, node).is_some() {
            "sym"
        } else {
            "dsym"
        };
    }
    match node.kind() {
        "identifier" => match node_bytes(source, node) {
            b"__FILE__" => "str",
            b"__LINE__" => "int",
            b"__ENCODING__" => "const",
            _ => "lvar",
        },
        "constant" | "scope_resolution" => "const",
        "string" | "chained_string" | "heredoc_beginning" => {
            if plain_string(source, node).is_some() {
                "str"
            } else {
                "dstr"
            }
        }
        "character" => "str",
        "call" => "csend",
        "string_array" | "symbol_array" => "array",
        "regex" => "regexp",
        "conditional" | "if" | "unless" | "if_modifier" | "unless_modifier" => "if",
        "binary" => {
            if node
                .child_by_field_name("operator")
                .is_some_and(|n| matches!(node_bytes(source, n), b"&&" | b"and"))
            {
                "and"
            } else {
                "or"
            }
        }
        "unary" => node.child_by_field_name("operand").map_or("defined?", |n| {
            if matches!(n.kind(), "rational" | "complex") {
                n.kind()
            } else {
                "defined?"
            }
        }),
        "assignment" => node
            .child_by_field_name("left")
            .map_or("lvasgn", |n| match n.kind() {
                "instance_variable" => "ivasgn",
                "class_variable" => "cvasgn",
                "global_variable" => "gvasgn",
                "constant" | "scope_resolution" => "casgn",
                "call" | "element_reference" => "send",
                "left_assignment_list" | "destructured_left_assignment" => "masgn",
                _ => "lvasgn",
            }),
        other => other,
    }
}
fn static_flag(source: &SourceFile, node: Node<'_>) -> bool {
    shorthand_value(source, node).is_none()
        && (matches!(
            node.kind(),
            "true" | "false" | "nil" | "integer" | "float" | "rational" | "complex" | "character"
        ) || node.kind() == "unary"
            && (integer_ast(source, node).is_some() || float_value(source, node).is_some())
            || matches!(
                node.kind(),
                "string" | "chained_string" | "heredoc_beginning"
            ) && crate::cop::ported::static_literal(source, node).is_some()
            || crate::cop::ported::static_symbol(source, node).is_some())
}
impl Cop for RedundantAllowNil {
    fn name(&self) -> &'static str {
        "Rails/RedundantAllowNil"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"validates"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !super::unknown_env::regular(node, source)
            || call_method_name(source, node) != Some(b"validates")
            || crate::cop::shared::is_setter_call(node)
        {
            return;
        }
        let Some((nil, blank)) = flags(node, true, source) else {
            return;
        };
        let Some(nil_value) = value(nil) else {
            return;
        };
        let Some(blank_value) = value(blank) else {
            return;
        };
        let nil_type = value_type(source, nil_value, config);
        let blank_type = value_type(source, blank_value, config);
        let message = if nil_type == blank_type {
            "`allow_nil` is redundant when `allow_blank` has the same value."
        } else if nil_type == "false" && blank_type == "true" {
            "`allow_nil: false` is redundant when `allow_blank` is true."
        } else {
            return;
        };
        let Some(parent) = nil.parent() else {
            return;
        };
        let siblings = children(parent);
        let Some(index) = siblings.iter().position(|n| *n == nil) else {
            return;
        };
        let (start, end) = if let Some(next) = siblings.get(index + 1) {
            (nil.start_byte(), next.start_byte())
        } else if index > 0 {
            (siblings[index - 1].end_byte(), nil.end_byte())
        } else {
            (nil.start_byte(), nil.end_byte())
        };
        let (line, col) = source.offset_to_line_col(nil.start_byte());
        let mut d = self.diagnostic(source, line, col, message.into());
        d.highlight_length =
            super::blank::highlight_length(source, nil.start_byte(), nil.end_byte());
        d.correctable = !config.autocorrect_disabled()
            && static_flag(source, nil_value)
            && static_flag(source, blank_value)
            && !collect_comments(parent)
                .iter()
                .any(|n| start < n.end_byte() && end > n.start_byte());
        let labels: Vec<_> = siblings
            .iter()
            .filter(|n| n.kind() == "pair")
            .filter_map(|n| n.child_by_field_name("key"))
            .map(|n| node_text(source, n))
            .collect();
        d.correctable &= labels
            .iter()
            .filter(|name| name.as_str() == "allow_nil")
            .count()
            == 1
            && labels
                .iter()
                .filter(|name| name.as_str() == "allow_blank")
                .count()
                == 1
            && !siblings.iter().any(|n| n.kind() == "hash_splat_argument");
        let mut ancestor = nil.parent();
        let mut depth = 0;
        while let Some(current) = ancestor.filter(|n| *n != node) {
            if current.kind() == "pair" {
                depth += 1;
            }
            d.correctable &= matches!(current.kind(), "pair" | "hash" | "argument_list")
                && depth <= 1
                && crate::cop::shared::call_receiver(node) != Some(current);
            ancestor = current.parent();
        }
        if d.correctable {
            let mut proposed = source.as_bytes().to_vec();
            proposed.drain(start..end);
            let proposed = SourceFile::from_bytes("allow_nil.rb", proposed);
            d.correctable =
                crate::parse::parse_ruby(&proposed).is_ok_and(|tree| !tree.root_node().has_error());
        }
        if d.correctable {
            d.corrected = push_replace(&mut corrections, start, end, "", self.name());
        }
        diagnostics.push(d);
    }
}
