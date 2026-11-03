//! Style/NilComparison: both original styles and selector locations.
use crate::cop::shared::{
    call_receiver, method_node, node_bytes, push_replace, semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::sync::OnceLock;
use tree_sitter::Node;

pub struct NilComparison;

fn comparison_pattern() -> &'static regex::bytes::Regex {
    static PATTERN: OnceLock<regex::bytes::Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        regex::bytes::Regex::new(r"[ \t\r\n\x0b\x0c]*={2,3}[ \t\r\n\x0b\x0c]*nil").unwrap()
    })
}

fn operator<'a>(source: &'a SourceFile, node: Node<'_>) -> &'a [u8] {
    node.child_by_field_name("operator")
        .map_or(b"", |n| node_bytes(source, n))
}

fn negation(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "unary" && matches!(operator(source, node), b"!" | b"not")
        || node.kind() == "call" && method_node(node).is_some_and(|m| node_bytes(source, m) == b"!")
}

fn defined_context(source: &SourceFile, mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        if parent.kind() == "parenthesized_statements" {
            node = parent;
        } else {
            return parent.kind() == "unary" && operator(source, parent) == b"defined?";
        }
    }
    false
}

fn signed_numeric_receiver(source: &SourceFile, node: Node<'_>, parent: Node<'_>) -> bool {
    if !matches!(operator(source, parent), b"+" | b"-") {
        return false;
    }
    let Some(sign) = parent.child_by_field_name("operator") else {
        return false;
    };
    let Some(mut receiver) = call_receiver(node) else {
        return false;
    };
    while receiver.kind() == "call" {
        let Some(inner) = call_receiver(receiver) else {
            return false;
        };
        receiver = inner;
    }
    matches!(
        receiver.kind(),
        "integer" | "float" | "rational" | "complex"
    ) && source.as_bytes()[sign.end_byte()..receiver.start_byte()]
        .iter()
        .all(u8::is_ascii_whitespace)
}

fn unsafe_comparison_context(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "call" | "element_reference" => {
            parent
                .child_by_field_name("receiver")
                .or_else(|| parent.child_by_field_name("object"))
                == Some(node)
                && !negation(source, parent)
        }
        "binary" => !matches!(operator(source, parent), b"&&" | b"||" | b"and" | b"or"),
        "unary" => !negation(source, parent) && !signed_numeric_receiver(source, node, parent),
        _ => false,
    }
}

impl Cop for NilComparison {
    fn name(&self) -> &'static str {
        "Style/NilComparison"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["binary", "call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let comparison = config.get_str("EnforcedStyle", "predicate") == "comparison";
        let (receiver, selector) = if node.kind() == "binary" {
            if comparison
                || !matches!(operator(source, node), b"==" | b"===")
                || !node
                    .child_by_field_name("right")
                    .is_some_and(|n| n.kind() == "nil")
            {
                return;
            }
            let (Some(receiver), Some(selector)) = (
                node.child_by_field_name("left"),
                node.child_by_field_name("operator"),
            ) else {
                return;
            };
            (receiver, selector)
        } else {
            if operator(source, node) == b"&." {
                return;
            }
            let (Some(receiver), Some(selector)) = (call_receiver(node), method_node(node)) else {
                return;
            };
            let args = semantic_argument_nodes(node);
            if comparison {
                if node_bytes(source, selector) != b"nil?" || !args.is_empty() {
                    return;
                }
            } else if !matches!(node_bytes(source, selector), b"==" | b"===")
                || args.len() != 1
                || args[0].kind() != "nil"
            {
                return;
            }
            (receiver, selector)
        };
        let (line, column) = source.offset_to_line_col(selector.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            if comparison {
                "Prefer the use of the `==` comparison."
            } else {
                "Prefer the use of the `nil?` predicate."
            }
            .into(),
        );
        diagnostic.highlight_length = node_bytes(source, selector).len();
        let bytes = source.as_bytes();
        let raw = &bytes[node.start_byte()..crate::cop::shared::send_end(node)];
        let mut safe = !defined_context(source, node);
        let edit = if comparison {
            raw.windows(5).position(|b| b == b".nil?").map(|start| {
                let start = node.start_byte() + start;
                safe &= start >= receiver.end_byte()
                    && node.child_by_field_name("arguments").is_none()
                    && node.child_by_field_name("block").is_none()
                    && !unsafe_comparison_context(source, node);
                (start, start + 5, " == nil")
            })
        } else {
            comparison_pattern().find(raw).map(|found| {
                let start = node.start_byte() + found.start();
                let end = node.start_byte() + found.end();
                safe &= node.kind() == "binary"
                    && start >= receiver.end_byte()
                    && bytes[receiver.end_byte()..start]
                        .iter()
                        .all(u8::is_ascii_whitespace);
                (start, end, ".nil?")
            })
        };
        if safe {
            if let Some((start, end, text)) = edit {
                diagnostic.corrected =
                    push_replace(&mut corrections, start, end, text, self.name());
            }
            let wrap = node.parent().is_some_and(|parent| negation(source, parent));
            if wrap {
                diagnostic.corrected |= push_replace(
                    &mut corrections,
                    node.start_byte(),
                    node.start_byte(),
                    "(",
                    self.name(),
                );
                let end = crate::cop::shared::send_end(node);
                diagnostic.corrected |= push_replace(&mut corrections, end, end, ")", self.name());
            }
            let mut normalize = false;
            for (start, window) in bytes.windows(2).enumerate() {
                if window == b"\r\n"
                    && !edit.is_some_and(|(left, right, _)| start >= left && start < right)
                {
                    normalize = true;
                    diagnostic.corrected |=
                        push_replace(&mut corrections, start, start + 1, "", self.name());
                }
            }
            diagnostic.correctable = edit.is_some() || wrap || normalize;
        } else {
            diagnostic.correctable = false;
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NilComparison, "cops/style/nil_comparison");
    #[test]
    fn nested_predicates_do_not_clobber() {
        let source = SourceFile::from_bytes("test.rb", b"x.nil?.nil?\n".to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig {
            options: [(
                "EnforcedStyle".into(),
                serde_yml::Value::String("comparison".into()),
            )]
            .into(),
            ..CopConfig::default()
        };
        let mut diagnostics = Vec::new();
        let mut corrections = Vec::new();
        crate::cop::walker::BatchedWalker::new(vec![&NilComparison], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut corrections),
        );
        assert_eq!(diagnostics.len(), 2);
        let corrected =
            crate::correction::CorrectionSet::from_vec(corrections).apply(source.as_bytes());
        assert_eq!(corrected, b"x.nil?.nil?\n");
        let rewritten = SourceFile::from_bytes("test.rb", corrected);
        assert!(
            !crate::parse::parse_ruby(&rewritten)
                .unwrap()
                .root_node()
                .has_error()
        );
    }
    #[test]
    fn original_diagnostics_with_safeguards() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/nil_comparison/intentional-safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&NilComparison, &cases.to_string());
    }
    #[test]
    fn independent_original_states() {
        crate::testutil::assert_original_spec_parity(
            &NilComparison,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/nil_comparison/independent-parity.json"
            )),
        );
    }
    #[test]
    fn original_ruby_states() {
        crate::testutil::assert_original_spec_parity(
            &NilComparison,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/nil_comparison/original-parity.json"
            )),
        );
    }
}
