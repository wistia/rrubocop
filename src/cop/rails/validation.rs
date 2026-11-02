use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, command_unary_plus_call,
    is_setter_call, method_node, named_kids, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct Validation;
const TYPES: &[&str] = &[
    "acceptance",
    "comparison",
    "confirmation",
    "exclusion",
    "format",
    "inclusion",
    "length",
    "numericality",
    "presence",
    "absence",
    "size",
    "uniqueness",
];
fn array(node: Node<'_>) -> bool {
    matches!(node.kind(), "array" | "string_array" | "symbol_array")
}
fn literal(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "string"
            | "chained_string"
            | "subshell"
            | "heredoc_beginning"
            | "character"
            | "integer"
            | "float"
            | "rational"
            | "complex"
            | "simple_symbol"
            | "delimited_symbol"
            | "array"
            | "string_array"
            | "symbol_array"
            | "hash"
            | "regex"
            | "true"
            | "false"
            | "nil"
            | "range"
    ) || node.kind() == "unary"
        && node
            .child_by_field_name("operator")
            .is_some_and(|n| matches!(node_text(source, n).as_str(), "+" | "-"))
        && node
            .child_by_field_name("operand")
            .is_some_and(|n| matches!(n.kind(), "integer" | "float" | "rational" | "complex"))
}
fn scalar_attribute(source: &SourceFile, node: Node<'_>) -> bool {
    literal(source, node) && !array(node) && node.kind() != "hash"
}
fn flat_splat(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() != "splat_argument" {
        return false;
    }
    let elements = named_kids(node);
    let [value] = elements.as_slice() else {
        return false;
    };
    array(*value)
        && named_kids(*value)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .all(|n| value.kind() != "array" || scalar_attribute(source, n))
}
fn frozen(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() == "identifier" {
        return node_text(source, node) == "freeze"
            && crate::cop::ported::bare_call_identifier(source, node);
    }
    node.kind() == "call"
        && call_method_name(source, node) == Some(b"freeze")
        && !is_setter_call(node)
        && node.child_by_field_name("block").is_none()
        && node
            .child_by_field_name("operator")
            .is_none_or(|n| node_text(source, n) != "&.")
}
fn valid_edit(
    source: &SourceFile,
    selector: Node<'_>,
    start: usize,
    end: usize,
    replacement: &str,
) -> bool {
    let mut bytes = source.as_bytes().to_vec();
    bytes.splice(start..end, replacement.bytes());
    bytes.splice(
        selector.start_byte()..selector.end_byte(),
        b"validates".iter().copied(),
    );
    let candidate = SourceFile::from_bytes(source.path.clone(), bytes);
    crate::parse::parse_ruby(&candidate)
        .ok()
        .is_some_and(|t| !t.root_node().has_error())
}
impl Cop for Validation {
    fn name(&self) -> &'static str {
        "Rails/Validation"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/models/**/*.rb"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[
            b"validates_acceptance_of",
            b"validates_comparison_of",
            b"validates_confirmation_of",
            b"validates_exclusion_of",
            b"validates_format_of",
            b"validates_inclusion_of",
            b"validates_length_of",
            b"validates_numericality_of",
            b"validates_presence_of",
            b"validates_absence_of",
            b"validates_size_of",
            b"validates_uniqueness_of",
        ]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let command = if node.kind() == "binary" {
            command_unary_plus_call(source, node)
        } else {
            None
        };
        let (selector, args, signed_last) = if let Some(command) = command.as_ref() {
            if command.receiver.is_some() {
                return;
            }
            (
                command.selector,
                command.arguments.iter().map(|a| a.node).collect::<Vec<_>>(),
                command.arguments.last().is_some_and(|a| a.unary_plus),
            )
        } else {
            if node.kind() != "call" || call_receiver(node).is_some() || is_setter_call(node) {
                return;
            }
            let Some(selector) = method_node(node) else {
                return;
            };
            (selector, argument_nodes(node), false)
        };
        let method = node_text(source, selector);
        let Some(kind) = method
            .strip_prefix("validates_")
            .and_then(|s| s.strip_suffix("_of"))
            .filter(|s| TYPES.contains(s))
        else {
            return;
        };
        let Some(&last) = args.last() else {
            return;
        };
        let implicit = matches!(last.kind(), "pair" | "hash_splat_argument");
        let frozen = !signed_last && frozen(source, last);
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let disabled = crate::parse::directives::cached(source).suppresses(self.name(), line);
        if !disabled
            && !implicit
            && !(literal(source, last)
                && (!signed_last
                    || matches!(last.kind(), "integer" | "float" | "rational" | "complex")))
            && !(last.kind() == "splat_argument" && last.named_child_count() > 0)
            && !frozen
        {
            return;
        }
        let mut d=self.diagnostic(source,line,col,format!("Prefer the new style validations `validates :column, {kind}: value` over `{method}`."));
        d.highlight_length = method.chars().count();
        d.correctable = false;
        if disabled {
            diagnostics.push(d);
            return;
        }
        let validation = if kind == "size" { "length" } else { kind };
        let mut edit = None;
        if implicit || last.kind() == "hash" {
            let (start, end, options) = if implicit {
                let first = args
                    .iter()
                    .rev()
                    .take_while(|n| matches!(n.kind(), "pair" | "hash_splat_argument"))
                    .last()
                    .copied()
                    .unwrap();
                let text = String::from_utf8_lossy(
                    &source.as_bytes()[first.start_byte()..last.end_byte()],
                );
                (first.start_byte(), last.end_byte(), format!("{{ {text} }}"))
            } else {
                (last.start_byte(), last.end_byte(), node_text(source, last))
            };
            edit = Some((start, end, format!("{validation}: {options}")));
        } else if array(last) || frozen {
            let attributes = if frozen {
                call_receiver(last)
            } else {
                Some(last)
            };
            if let Some(attributes) = attributes.filter(|n| array(*n)) {
                let children: Vec<_> = named_kids(attributes)
                    .into_iter()
                    .filter(|n| n.kind() != "comment")
                    .collect();
                if !children.is_empty()
                    && children.iter().all(|n| {
                        attributes.kind() != "array"
                            || literal(source, *n) && !array(*n) && n.kind() != "hash"
                    })
                    && collect_comments(last).is_empty()
                    && (!frozen || argument_nodes(last).is_empty())
                {
                    let values = children
                        .iter()
                        .map(|n| {
                            if attributes.kind() == "array" {
                                node_text(source, *n)
                            } else {
                                format!(":{}", node_text(source, *n))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    edit = Some((
                        last.start_byte(),
                        last.end_byte(),
                        format!("{values}, {validation}: true"),
                    ));
                }
            }
        } else {
            edit = Some((
                last.end_byte(),
                last.end_byte(),
                format!(", {validation}: true"),
            ));
        }
        let attribute_end = if implicit {
            args.iter()
                .rposition(|n| !matches!(n.kind(), "pair" | "hash_splat_argument"))
                .map_or(0, |i| i + 1)
        } else if last.kind() == "hash" {
            args.len() - 1
        } else {
            args.len()
        };
        let safe_attributes = args[..attribute_end]
            .iter()
            .enumerate()
            .all(|(index, arg)| {
                index + 1 == args.len() && (array(*arg) || frozen)
                    || scalar_attribute(source, *arg)
                    || flat_splat(source, *arg)
            });
        if let Some((start, end, replacement)) = edit {
            d.correctable = !config.autocorrect_disabled()
                && safe_attributes
                && valid_edit(source, selector, start, end, &replacement);
            if d.correctable {
                let renamed = push_replace(
                    &mut corrections,
                    selector.start_byte(),
                    selector.end_byte(),
                    "validates",
                    self.name(),
                );
                let changed = push_replace(&mut corrections, start, end, replacement, self.name());
                d.corrected = renamed && changed;
            }
        }
        diagnostics.push(d);
    }
}
