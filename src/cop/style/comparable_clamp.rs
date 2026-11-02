//! Style/ComparableClamp.
use crate::cop::shared::{
    call_method_name, call_receiver, integer_value, node_bytes, push_replace,
    semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ComparableClamp;
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|node| {
            !matches!(
                node.kind(),
                "comment" | "heredoc_body" | "heredoc_end" | "empty_statement"
            )
        })
        .collect()
}
fn statement(node: Node<'_>) -> Option<Node<'_>> {
    if matches!(
        node.kind(),
        "then" | "else" | "body_statement" | "block_body"
    ) {
        let kids = children(node);
        if kids.len() == 1 {
            return statement(kids[0]);
        }
        return None;
    }
    Some(node)
}
fn branches(node: Node<'_>) -> Option<(Node<'_>, Node<'_>, Node<'_>)> {
    if !matches!(node.kind(), "if" | "unless" | "elsif" | "conditional") {
        return None;
    }
    let condition = node.child_by_field_name("condition")?;
    let mut yes = statement(node.child_by_field_name("consequence")?)?;
    let mut no = statement(node.child_by_field_name("alternative")?)?;
    if node.kind() == "unless" {
        std::mem::swap(&mut yes, &mut no);
    }
    Some((condition, yes, no))
}
fn comparison<'s, 'n>(
    source: &'s SourceFile,
    node: Node<'n>,
) -> Option<(Node<'n>, &'s [u8], Node<'n>)> {
    if node.kind() == "binary" {
        let op = node.child_by_field_name("operator")?;
        let op = node_bytes(source, op);
        return matches!(op, b"<" | b">").then(|| {
            (
                node.child_by_field_name("left").unwrap(),
                op,
                node.child_by_field_name("right").unwrap(),
            )
        });
    }
    if node.kind() == "call"
        && !node
            .child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"&.")
        && node.child_by_field_name("block").is_none()
    {
        let method = call_method_name(source, node)?;
        let args = semantic_argument_nodes(node);
        if matches!(method, b"<" | b">") && args.len() == 1 {
            return Some((call_receiver(node)?, method, args[0]));
        }
    }
    None
}
fn equal(source: &SourceFile, left: Node<'_>, right: Node<'_>) -> bool {
    crate::cop::shared::ruby_expression::same_expression(source, left, right)
}
fn direction(
    source: &SourceFile,
    condition: Node<'_>,
    bound: Node<'_>,
    value: Node<'_>,
) -> Option<bool> {
    let (left, op, right) = comparison(source, condition)?;
    if equal(source, left, value) && equal(source, right, bound) {
        return Some(op == b"<");
    }
    if equal(source, left, bound) && equal(source, right, value) {
        return Some(op == b">");
    }
    None
}
fn if_match<'n>(
    source: &SourceFile,
    node: Node<'n>,
) -> Option<(Node<'n>, Node<'n>, Node<'n>, bool)> {
    let (condition, first, alternative) = branches(node)?;
    let (other_condition, second, value) = branches(alternative)?;
    let first_min = direction(source, condition, first, value)?;
    if direction(source, other_condition, second, value)? == first_min {
        return None;
    }
    Some((value, first, second, first_min))
}
fn ordinary_call(source: &SourceFile, node: Node<'_>, method: &[u8]) -> bool {
    node.kind() == "call"
        && call_method_name(source, node) == Some(method)
        && semantic_argument_nodes(node).is_empty()
        && !node
            .child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"&.")
}
fn array_match(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(method) =
        call_method_name(source, node).filter(|method| matches!(*method, b"min" | b"max"))
    else {
        return false;
    };
    if !ordinary_call(source, node, method) {
        return false;
    }
    let Some(array) = call_receiver(node).filter(|node| node.kind() == "array") else {
        return false;
    };
    let items = children(array);
    if items.len() != 2 {
        return false;
    }
    let inverse = if method == b"min" {
        b"max".as_slice()
    } else {
        b"min".as_slice()
    };
    items.into_iter().any(|item| {
        ordinary_call(source, item, inverse)
            && item.child_by_field_name("block").is_none()
            && call_receiver(item)
                .is_some_and(|array| array.kind() == "array" && children(array).len() == 2)
    })
}
fn receiver_safe(source: &SourceFile, node: Node<'_>) -> bool {
    if matches!(
        node.kind(),
        "binary" | "unary" | "conditional" | "if" | "unless" | "elsif"
    ) {
        return false;
    }
    if node.kind() == "call" && !semantic_argument_nodes(node).is_empty() {
        return node
            .child_by_field_name("arguments")
            .is_some_and(|args| node_bytes(source, args).starts_with(b"("));
    }
    true
}
impl Cop for ComparableClamp {
    fn name(&self) -> &'static str {
        "Style/ComparableClamp"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "if", "unless", "elsif", "conditional"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_f64("TargetRubyVersion", 2.7) < 2.4 {
            return;
        }
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut end = node
            .child_by_field_name("block")
            .map_or(node.end_byte(), |block| block.start_byte());
        while end > node.start_byte() && source.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        let mut diagnostic = if let Some((value, first, second, actual_min)) =
            if_match(source, node)
        {
            let condition = branches(node).unwrap().0;
            let (left, operator, right) = comparison(source, condition).unwrap();
            let source_min = (node_bytes(source, left) == node_bytes(source, value)
                && operator == b"<")
                || (node_bytes(source, right) == node_bytes(source, value) && operator == b">");
            let (min, max) = if source_min {
                (first, second)
            } else {
                (second, first)
            };
            let prefer = format!(
                "{}.clamp({}, {})",
                String::from_utf8_lossy(node_bytes(source, value)),
                String::from_utf8_lossy(node_bytes(source, min)),
                String::from_utf8_lossy(node_bytes(source, max))
            );
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                format!("Use `{prefer}` instead of `if/elsif/else`."),
            );
            let reversed = integer_value(source, min)
                .zip(integer_value(source, max))
                .is_some_and(|(min, max)| min > max);
            let retained = [value, min, max];
            let drops_comment = crate::cop::shared::collect_comments(node)
                .iter()
                .any(|comment| {
                    !retained.iter().any(|part| {
                        part.start_byte() <= comment.start_byte()
                            && comment.end_byte() <= part.end_byte()
                    })
                });
            let invalid_utf8 = retained
                .iter()
                .any(|part| std::str::from_utf8(node_bytes(source, *part)).is_err());
            if source_min != actual_min
                || reversed
                || !receiver_safe(source, value)
                || drops_comment
                || invalid_utf8
                || retained
                    .iter()
                    .any(|part| part.kind() == "heredoc_beginning")
            {
                diagnostic.correctable = false;
            } else {
                let replacement = if node.kind() == "elsif" {
                    let indent = " ".repeat(column + config.get_usize("IndentationWidth", 2));
                    format!("else\n{indent}{prefer}")
                } else {
                    prefer
                };
                diagnostic.corrected = push_replace(
                    &mut corrections,
                    node.start_byte(),
                    node.end_byte(),
                    replacement,
                    self.name(),
                );
            }
            diagnostic
        } else if array_match(source, node) {
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Use `Comparable#clamp` instead.".into(),
            );
            diagnostic.correctable = false;
            diagnostic
        } else {
            return;
        };
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                .chars()
                .count();
        diagnostics.push(diagnostic);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_diagnostics_with_intentional_safeguards() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/comparable_clamp/intentional-safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&ComparableClamp, &cases.to_string());
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &ComparableClamp,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/comparable_clamp/parity.json"
            )),
        );
    }
}
