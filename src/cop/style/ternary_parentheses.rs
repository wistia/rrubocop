//! Style/TernaryParentheses.

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TernaryParentheses;

fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .collect()
}

fn parenthesized(node: Node<'_>) -> bool {
    node.kind() == "parenthesized_statements"
}

fn safe_assignment(source: &SourceFile, node: Node<'_>) -> bool {
    if !parenthesized(node) {
        return false;
    }
    let children = children(node);
    if children.len() != 1 {
        return false;
    }
    let child = children[0];
    matches!(child.kind(), "assignment" | "operator_assignment")
        || (child.kind() == "call"
            && call_method_name(source, child).is_some_and(|method| {
                method.ends_with(b"=")
                    && !matches!(
                        method,
                        b"==" | b"!=" | b">=" | b"<=" | b"===" | b"<=>" | b"=~"
                    )
            }))
}

fn defined(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "unary"
        && node
            .child_by_field_name("operator")
            .is_some_and(|operator| node_bytes(source, operator) == b"defined?")
}

fn complex(source: &SourceFile, node: Node<'_>) -> bool {
    if parenthesized(node) {
        return children(node).iter().any(|child| complex(source, *child));
    }
    if matches!(
        node.kind(),
        "identifier"
            | "instance_variable"
            | "class_variable"
            | "global_variable"
            | "constant"
            | "scope_resolution"
            | "yield"
            | "element_reference"
    ) || defined(source, node)
    {
        return false;
    }
    if node.kind() == "call" {
        if node.child_by_field_name("block").is_some() {
            return true;
        }
        let Some(method) = call_method_name(source, node) else {
            return true;
        };
        return method != b"[]"
            && method
                .first()
                .is_some_and(|byte| !byte.is_ascii_alphabetic() && *byte != b'_');
    }
    true
}

fn unsafe_omission(source: &SourceFile, condition: Node<'_>) -> bool {
    children(condition).iter().any(|child| {
        if child.kind() == "binary"
            && child
                .child_by_field_name("operator")
                .is_some_and(|operator| matches!(node_bytes(source, operator), b"and" | b"or"))
        {
            return true;
        }
        if child.kind() == "unary"
            && child
                .child_by_field_name("operator")
                .is_some_and(|operator| node_bytes(source, operator) == b"not")
        {
            return true;
        }
        if defined(source, *child) {
            return !child
                .child_by_field_name("operand")
                .is_some_and(parenthesized);
        }
        if child.kind() == "call"
            && !argument_nodes(*child).is_empty()
            && call_method_name(source, *child)
                .is_some_and(|method| method.first().is_some_and(u8::is_ascii_alphabetic))
        {
            return !child
                .child_by_field_name("arguments")
                .is_some_and(|arguments| node_bytes(source, arguments).starts_with(b"("));
        }
        false
    })
}

fn skip_condition(source: &SourceFile, condition: Node<'_>) -> bool {
    if node_bytes(source, condition)
        .split(|byte| *byte == b'\n')
        .last()
        == Some(&b")"[..])
    {
        return true;
    }
    parenthesized(condition)
        && children(condition)
            .first()
            .is_some_and(|child| matches!(child.kind(), "match_pattern" | "test_pattern"))
}

impl Cop for TernaryParentheses {
    fn name(&self) -> &'static str {
        "Style/TernaryParentheses"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["conditional"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(mut condition) = node.child_by_field_name("condition") else {
            return;
        };
        if condition.kind() == "unary"
            && condition
                .child_by_field_name("operator")
                .is_some_and(|operator| node_bytes(source, operator) == b"not")
        {
            condition = condition
                .child_by_field_name("operand")
                .unwrap_or(condition);
        }
        if skip_condition(source, condition) {
            return;
        }
        let parens = parenthesized(condition);
        let assignment = safe_assignment(source, condition);
        let style = config.get_str("EnforcedStyle", "require_no_parentheses");
        let offense = if assignment {
            !config.get_bool("AllowSafeAssignment", true)
        } else {
            match style {
                "require_parentheses" => !parens,
                "require_parentheses_when_complex" => complex(source, condition) != parens,
                _ => parens,
            }
        };
        if !offense {
            return;
        }
        let message = if style == "require_parentheses_when_complex" {
            format!(
                "{} parentheses for ternary expressions with complex conditions.",
                if parens { "Only use" } else { "Use" }
            )
        } else {
            format!(
                "{} parentheses for ternary conditions.",
                if style == "require_parentheses" {
                    "Use"
                } else {
                    "Omit"
                }
            )
        };
        let (line, column) = source.offset_to_line_col(condition.start_byte());
        let mut diagnostic = self.diagnostic(source, line, column, message);
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[condition.start_byte()..node.end_byte()])
                .chars()
                .count();
        if parens && (assignment || unsafe_omission(source, condition)) {
            diagnostic.correctable = false;
        } else if parens {
            diagnostic.corrected = push_replace(
                &mut corrections,
                condition.start_byte(),
                condition.start_byte() + 1,
                "",
                self.name(),
            );
            let end = condition.end_byte();
            let space = if source
                .as_bytes()
                .get(end)
                .is_some_and(u8::is_ascii_whitespace)
            {
                ""
            } else {
                " "
            };
            push_replace(&mut corrections, end - 1, end, space, self.name());
        } else {
            diagnostic.corrected = push_replace(
                &mut corrections,
                condition.start_byte(),
                condition.start_byte(),
                "(",
                self.name(),
            );
            push_replace(
                &mut corrections,
                condition.end_byte(),
                condition.end_byte(),
                ")",
                self.name(),
            );
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn does_not_move_a_safe_navigation_argument_into_a_ternary() {
        let diagnostics = crate::testutil::run_cop_full(
            &TernaryParentheses,
            b"(object&.predicate argument) ? a : b\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }

    #[test]
    fn original_block_condition_roles() {
        crate::testutil::assert_original_spec_parity(
            &TernaryParentheses,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/ternary_parentheses/block-conditions.json"
            )),
        );
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &TernaryParentheses,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/ternary_parentheses/parity.json"
            )),
        );
    }
}
