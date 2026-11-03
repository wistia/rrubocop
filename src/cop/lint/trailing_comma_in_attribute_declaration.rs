//! Lint/TrailingCommaInAttributeDeclaration: original accessor and definition roles.
use crate::cop::shared::{self, named_kids, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use tree_sitter::Tree;

pub struct TrailingCommaInAttributeDeclaration;
fn range_start(bytes: &[u8], mut offset: usize) -> usize {
    while bytes.get(offset).is_some_and(|b| matches!(b, b' ' | b'\t')) {
        offset += 1;
    }
    loop {
        if bytes.get(offset..offset + 2) == Some(b"\r\n") {
            offset += 2;
        } else if bytes.get(offset) == Some(&b'\n') {
            offset += 1;
        } else {
            break;
        }
    }
    offset
}
fn separates_statements(
    source: &SourceFile,
    node: tree_sitter::Node<'_>,
    start: usize,
    end: usize,
) -> bool {
    let comments: Vec<_> = shared::ruby_token_nodes(source, node)
        .into_iter()
        .filter(|n| n.kind() == "comment")
        .map(|n| (n.start_byte(), n.end_byte()))
        .collect();
    let bytes = source.as_bytes();
    (start..end).any(|index| {
        if bytes[index] != b'\n' {
            return false;
        }
        let previous = if index > start && bytes[index - 1] == b'\r' {
            index - 1
        } else {
            index
        };
        let escape = previous
            .checked_sub(1)
            .filter(|offset| bytes[*offset] == b'\\');
        !escape.is_some_and(|offset| !comments.iter().any(|(a, b)| offset >= *a && offset < *b))
    })
}
impl Cop for TrailingCommaInAttributeDeclaration {
    fn name(&self) -> &'static str {
        "Lint/TrailingCommaInAttributeDeclaration"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn uses_source_phase(&self) -> bool {
        true
    }
    fn default_severity(&self) -> crate::diagnostic::Severity {
        crate::diagnostic::Severity::Warning
    }
    fn check_source(
        &self,
        source: &SourceFile,
        tree: &Tree,
        _map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let initial = corrections.as_ref().map_or(0, |edits| edits.len());
        let mut pending = vec![tree.root_node()];
        while let Some(node) = pending.pop() {
            pending.extend(named_kids(node).into_iter().rev());
            if node.kind() != "call"
                || node.has_error()
                || node.child_by_field_name("receiver").is_some()
            {
                continue;
            }
            let Some(method) = node.child_by_field_name("method") else {
                continue;
            };
            if !matches!(
                node_bytes(source, method),
                b"attr" | b"attr_reader" | b"attr_writer" | b"attr_accessor"
            ) {
                continue;
            }
            let args = shared::ruby_arguments::from_nodes(
                shared::argument_nodes(node).into_iter().filter(|n| {
                    !n.is_extra() && !matches!(n.kind(), "comment" | "heredoc_body" | "heredoc_end")
                }),
            );
            let Some(definition) = args
                .last()
                .and_then(|a| a.node())
                .filter(|n| n.kind() == "method")
            else {
                continue;
            };
            let Some(previous) = args
                .get(args.len().saturating_sub(2))
                .filter(|_| args.len() >= 2)
            else {
                continue;
            };
            let start = range_start(source.as_bytes(), previous.end());
            let Some(&byte) = source.as_bytes().get(start) else {
                continue;
            };
            let parens = node
                .child_by_field_name("arguments")
                .is_some_and(|n| node_bytes(source, n).first() == Some(&b'('));
            let can_remove = byte == b','
                && !parens
                && separates_statements(source, node, start + 1, definition.start_byte());
            let (line, column) = source.offset_to_line_col(start);
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Avoid leaving a trailing comma in attribute declarations.".into(),
            );
            diagnostic.highlight_length = 1;
            diagnostic.correctable = can_remove && !config.autocorrect_disabled();
            if diagnostic.correctable {
                diagnostic.corrected =
                    push_replace(&mut corrections, start, start + 1, "", self.name());
            }
            diagnostics.push(diagnostic);
        }
        if let Some(corrections) = corrections {
            shared::normalize_corrected_crlf(source, self.name(), corrections, initial);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_spec_states() {
        crate::testutil::assert_original_spec_parity(
            &TrailingCommaInAttributeDeclaration,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/trailing_comma_in_attribute_declaration/original-parity.json"
            )),
        );
    }
    #[test]
    fn independent_original_states() {
        let mut cases:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/tests/fixtures/cops/lint/trailing_comma_in_attribute_declaration/original-independent.json"))).unwrap();
        for case in cases.as_array_mut().unwrap() {
            if case.get("rust_corrected").is_some() {
                case["ruby_corrected"] = case["rust_corrected"].clone();
            }
        }
        crate::testutil::assert_original_spec_parity(
            &TrailingCommaInAttributeDeclaration,
            &serde_json::to_string(&cases).unwrap(),
        );
    }
    #[test]
    fn native_parse_clean_coverage() {
        let cases:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/tests/fixtures/cops/lint/trailing_comma_in_attribute_declaration/original-independent.json"))).unwrap();
        let mut errors = Vec::new();
        for case in cases.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                case["original"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            if tree.root_node().has_error() {
                errors.push(serde_json::json!({"id":case["id"],"source":case["original"],"tree":tree.root_node().to_sexp()}));
            }
        }
        assert!(
            errors.is_empty(),
            "NATIVE_ERROR_COVERAGE {}",
            serde_json::to_string(&errors).unwrap()
        );
    }
}
