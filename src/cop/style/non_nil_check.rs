//! Style/NonNilCheck: original configuration, predicate exemptions and corrections.
use crate::cop::shared::{
    call_method_name, call_receiver, named_kids, node_bytes, push_replace, semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::sync::LazyLock;
use tree_sitter::Node;
pub struct NonNilCheck;
static COMPARISON: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"[\t\n\r\x0b\x0c ]*!=[\t\n\r\x0b\x0c ]*nil").unwrap());
static PREDICATE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^([^\t\n\r\x0b\x0c ]*)[\t\n\r\x0b\x0c ]*!=[\t\n\r\x0b\x0c ]*nil").unwrap()
});
fn statements(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| !matches!(n.kind(), "comment" | "empty_statement"))
        .collect()
}
fn predicate_result(source: &SourceFile, node: Node<'_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if matches!(parent.kind(), "method" | "singleton_method") {
            if !parent
                .child_by_field_name("name")
                .is_some_and(|n| node_bytes(source, n).ends_with(b"?"))
            {
                return false;
            }
            let Some(body) = parent.child_by_field_name("body") else {
                return false;
            };
            let parts = if matches!(body.kind(), "body_statement" | "block_body") {
                statements(body)
            } else {
                vec![body]
            };
            let Some(mut last) = parts.last().copied() else {
                return false;
            };
            if parts.len() == 1 && last.kind() == "parenthesized_statements" {
                let Some(inner) = statements(last).last().copied() else {
                    return false;
                };
                last = inner;
            }
            return node == last;
        }
        if matches!(parent.kind(), "class" | "module" | "singleton_class") {
            return false;
        }
        ancestor = parent.parent();
    }
    false
}
fn nil_call<'n>(source: &SourceFile, node: Node<'n>) -> Option<Option<Node<'n>>> {
    if node.kind() == "identifier" && node_bytes(source, node) == b"nil?" {
        return (!node
            .parent()
            .is_some_and(|p| p.child_by_field_name("method") == Some(node)))
        .then_some(None);
    }
    if node.kind() != "call"
        || call_method_name(source, node) != Some(b"nil?")
        || !semantic_argument_nodes(node).is_empty()
        || node.child_by_field_name("block").is_some()
        || node
            .child_by_field_name("operator")
            .is_some_and(|n| node_bytes(source, n) == b"&.")
    {
        return None;
    }
    Some(call_receiver(node))
}
impl Cop for NonNilCheck {
    fn name(&self) -> &'static str {
        "Style/NonNilCheck"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["binary", "unary", "call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let semantic = config.get_bool("IncludeSemanticChanges", false);
        if !semantic
            && config.get_bool("NilComparisonEnabled", true)
            && config.get_str("NilComparisonEnforcedStyle", "predicate") == "comparison"
            || predicate_result(source, node)
        {
            return;
        }
        let mut comparison = None;
        if node.kind() == "binary"
            && node
                .child_by_field_name("operator")
                .is_some_and(|n| node_bytes(source, n) == b"!=")
        {
            if let (Some(left), Some(right)) = (
                node.child_by_field_name("left"),
                node.child_by_field_name("right"),
            ) {
                if right.kind() == "nil" {
                    comparison = Some((left, right));
                }
            }
        } else if node.kind() == "call"
            && call_method_name(source, node) == Some(b"!=")
            && !node
                .child_by_field_name("operator")
                .is_some_and(|n| node_bytes(source, n) == b"&.")
        {
            let args = semantic_argument_nodes(node);
            if args.len() == 1 && args[0].kind() == "nil" {
                comparison = call_receiver(node).map(|left| (left, args[0]));
            }
        }
        let mut replacement = None;
        let mut keyword = None;
        let message = if let Some((receiver, right)) = comparison {
            let raw = node_bytes(source, node);
            let current = String::from_utf8_lossy(raw);
            if let Ok(text) = std::str::from_utf8(raw) {
                if semantic {
                    if let Some(found) = COMPARISON.find(text) {
                        if node.start_byte() + found.end() == right.end_byte() {
                            replacement = Some(format!(
                                "{}{}",
                                &text[..found.start()],
                                &text[found.end()..]
                            ));
                        }
                    }
                } else if !matches!(receiver.kind(), "binary" | "unary" | "conditional") {
                    if let Some(captures) = PREDICATE.captures(text) {
                        let whole = captures.get(0).unwrap();
                        if node.start_byte() + whole.end() == right.end_byte() {
                            replacement =
                                Some(format!("!{}.nil?{}", &captures[1], &text[whole.end()..]));
                        }
                    }
                }
            }
            if semantic {
                "Explicit non-nil checks are usually redundant.".into()
            } else {
                format!(
                    "Prefer `!{}.nil?` over `{current}`.",
                    String::from_utf8_lossy(node_bytes(source, receiver))
                )
            }
        } else if semantic
            && (node.kind() == "unary"
                && node
                    .child_by_field_name("operator")
                    .is_some_and(|n| matches!(node_bytes(source, n), b"!" | b"not"))
                || node.kind() == "call"
                    && call_method_name(source, node) == Some(b"!")
                    && semantic_argument_nodes(node).is_empty()
                    && !node
                        .child_by_field_name("operator")
                        .is_some_and(|n| node_bytes(source, n) == b"&."))
        {
            let Some(mut inner) = node
                .child_by_field_name("operand")
                .or_else(|| call_receiver(node))
            else {
                return;
            };
            if inner.kind() == "parenthesized_statements"
                && node
                    .child_by_field_name("operator")
                    .is_some_and(|op| node_bytes(source, op) == b"not")
            {
                let parts = statements(inner);
                if parts.len() == 1 {
                    inner = parts[0];
                }
            }
            let Some(receiver) = nil_call(source, inner) else {
                return;
            };
            replacement = Some(receiver.map_or_else(
                || "self".into(),
                |n| String::from_utf8_lossy(node_bytes(source, n)).into_owned(),
            ));
            "Explicit non-nil checks are usually redundant.".into()
        } else if semantic {
            let Some(receiver) = nil_call(source, node) else {
                return;
            };
            let Some(parent) = node.parent().filter(|p| {
                matches!(p.kind(), "unless" | "unless_modifier")
                    && p.child_by_field_name("condition") == Some(node)
            }) else {
                return;
            };
            if let Some(receiver) = receiver {
                replacement = std::str::from_utf8(node_bytes(source, receiver))
                    .ok()
                    .map(str::to_owned);
            }
            keyword = parent
                .child(0)
                .filter(|n| node_bytes(source, *n) == b"unless")
                .or_else(|| {
                    let mut cursor = parent.walk();
                    parent
                        .children(&mut cursor)
                        .find(|n| node_bytes(source, *n) == b"unless")
                });
            if keyword.is_none() {
                replacement = None;
            }
            "Explicit non-nil checks are usually redundant.".into()
        } else {
            return;
        };
        if !crate::cop::shared::collect_comments(node).is_empty()
            || std::str::from_utf8(node_bytes(source, node)).is_err()
            || comparison.is_some() && node.kind() == "call"
            || node.child_by_field_name("block").is_some()
        {
            replacement = None;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(source, line, col, message);
        let mut end = node
            .child_by_field_name("block")
            .map_or(node.end_byte(), |n| n.start_byte());
        while end > node.start_byte() && source.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                .chars()
                .count();
        if let Some(replacement) = replacement.filter(|s| s.as_bytes() != node_bytes(source, node))
        {
            diagnostic.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                node.end_byte(),
                replacement,
                self.name(),
            );
            if let Some(keyword) = keyword {
                push_replace(
                    &mut corrections,
                    keyword.start_byte(),
                    keyword.end_byte(),
                    "if",
                    self.name(),
                );
            }
        } else {
            diagnostic.correctable = false;
        }
        diagnostics.push(diagnostic);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_diagnostics_with_safeguards() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/non_nil_check/intentional-safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&NonNilCheck, &cases.to_string());
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &NonNilCheck,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/non_nil_check/parity.json"
            )),
        );
    }
}
