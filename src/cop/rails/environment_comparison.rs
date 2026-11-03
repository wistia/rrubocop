use super::unknown_env::{rails_env, regular};
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, node_text, push_replace,
    send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct EnvironmentComparison;
const EXISTING_PREDICATES: &[&str] = &[
    "acts_like",
    "acts_like_string",
    "ascii_only",
    "between",
    "blank",
    "casecmp",
    "duplicable",
    "empty",
    "end_with",
    "ends_with",
    "eql",
    "equal",
    "exclude",
    "frozen",
    "html_safe",
    "in",
    "include",
    "instance_of",
    "instance_variable_defined",
    "is_a",
    "is_utf8",
    "kind_of",
    "match",
    "nil",
    "present",
    "respond_to",
    "start_with",
    "starts_with",
    "unicode_normalized",
    "valid_encoding",
];
fn predicate_name(value: &str) -> bool {
    if EXISTING_PREDICATES.contains(&value) {
        return false;
    }
    let text = format!("Rails.env.{value}?");
    let source = SourceFile::from_bytes("predicate.rb", text.as_bytes().to_vec());
    crate::parse::parse_ruby(&source).ok().is_some_and(|tree| {
        let root = tree.root_node();
        !root.has_error()
            && root.named_child_count() == 1
            && root.named_child(0).is_some_and(|node| {
                node.kind() == "call"
                    && node.end_byte() == text.len()
                    && call_receiver(node).is_some_and(|receiver| rails_env(&source, receiver))
                    && call_method_name(&source, node) == Some(format!("{value}?").as_bytes())
            })
    })
}
impl Cop for EnvironmentComparison {
    fn name(&self) -> &'static str {
        "Rails/EnvironmentComparison"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let (left, right, negative) = if node.kind() == "binary" {
            let Some(op) = node.child_by_field_name("operator") else {
                return;
            };
            let op = node_text(source, op);
            if !matches!(op.as_str(), "==" | "!=") {
                return;
            }
            let (Some(left), Some(right)) = (
                node.child_by_field_name("left"),
                node.child_by_field_name("right"),
            ) else {
                return;
            };
            (left, right, op == "!=")
        } else if regular(node, source)
            && matches!(call_method_name(source, node), Some(b"==" | b"!="))
        {
            let args = argument_nodes(node);
            let Some(left) = call_receiver(node) else {
                return;
            };
            if args.len() != 1 {
                return;
            }
            (left, args[0], call_method_name(source, node) == Some(b"!="))
        } else {
            return;
        };
        let (env, literal) = if rails_env(source, left) {
            (left, right)
        } else if rails_env(source, right) {
            (right, left)
        } else {
            return;
        };
        let symbol = matches!(literal.kind(), "simple_symbol" | "delimited_symbol");
        let value = if symbol {
            let Some(parts) = crate::cop::ported::ruby_string_parts(source, literal)
                .or_else(|| crate::cop::ported::static_symbol(source, literal).map(|v| vec![v]))
            else {
                return;
            };
            if parts.len() != 1 {
                return;
            }
            parts[0].clone()
        } else if matches!(literal.kind(), "string" | "character" | "heredoc_beginning") {
            let Some(parts) = crate::cop::ported::ruby_string_parts(source, literal) else {
                return;
            };
            if parts.len() != 1 {
                return;
            }
            parts[0].clone()
        } else {
            return;
        };
        let end = if node.kind() == "call" {
            send_end(node)
        } else {
            node.end_byte()
        };
        let original =
            std::str::from_utf8(&source.as_bytes()[node.start_byte()..end]).unwrap_or_default();
        let bang = if negative { "!" } else { "" };
        let message = if symbol {
            "Do not compare `Rails.env` with a symbol, it will always evaluate to `false`.".into()
        } else {
            format!("Favor `{bang}Rails.env.{value}?` over `{original}`.")
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(source, line, col, message);
        d.highlight_length = super::blank::highlight_length(source, node.start_byte(), end);
        d.correctable = !config.autocorrect_disabled()
            && !symbol
            && literal.kind() != "heredoc_beginning"
            && !(value == "local" && config.get_f64("TargetRailsVersion", 8.1) >= 7.1)
            && predicate_name(&value)
            && collect_comments(node).is_empty();
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                end,
                format!("{bang}{}.{value}?", node_text(source, env)),
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicate_requires_one_unshadowed_method() {
        assert!(predicate_name("production"));
        assert!(predicate_name("staging"));
        for name in [
            "",
            "not-an-env",
            "production?",
            "foo.bar",
            "nil",
            "empty",
            "blank",
            "present",
            "respond_to",
        ] {
            assert!(!predicate_name(name), "{name:?}");
        }
    }
}
