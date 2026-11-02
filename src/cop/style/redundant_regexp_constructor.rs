//! Style/RedundantRegexpConstructor.
use crate::cop::shared::{
    call_method_name, call_receiver, node_bytes, push_replace, semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct RedundantRegexpConstructor;
impl Cop for RedundantRegexpConstructor {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpConstructor"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) =
            call_method_name(source, node).filter(|method| matches!(*method, b"new" | b"compile"))
        else {
            return;
        };
        if node
            .child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"&.")
            || !call_receiver(node)
                .is_some_and(|recv| matches!(node_bytes(source, recv), b"Regexp" | b"::Regexp"))
        {
            return;
        }
        let args = semantic_argument_nodes(node);
        if args.len() != 1 || args[0].kind() != "regex" {
            return;
        }
        let mut end = node
            .child_by_field_name("block")
            .map_or(node.end_byte(), |block| block.start_byte());
        while end > node.start_byte() && source.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!(
                "Remove the redundant `Regexp.{}`.",
                String::from_utf8_lossy(method)
            ),
        );
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                .chars()
                .count();
        let drops_comment = crate::cop::shared::collect_comments(node)
            .iter()
            .any(|comment| {
                comment.start_byte() < args[0].start_byte()
                    || comment.end_byte() > args[0].end_byte()
            });
        if node.child_by_field_name("block").is_some() || drops_comment {
            diagnostic.correctable = false;
        } else if let Some(replacement) =
            crate::cop::ported::redundant_regexp_literal(source, args[0])
        {
            diagnostic.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                node.end_byte(),
                replacement,
                self.name(),
            );
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
    fn preserves_source_for_unsafe_original_corrections() {
        for input in [
            b"Regexp.new(%r{a/b})".as_slice(),
            b"Regexp.new(%r{[/]})",
            b"Regexp.new(/x/) { nil }",
            b"Regexp.compile(/x/ # retain\n)",
            b"# coding: ASCII-8BIT\nRegexp.new(/\xff/)\n",
            b"Regexp.new(%r{# / comment\n}x)",
        ] {
            let diagnostics = crate::testutil::run_cop_full(&RedundantRegexpConstructor, input);
            assert_eq!(diagnostics.len(), 1, "{input:?}");
            assert!(!diagnostics[0].correctable, "{input:?}");
        }
    }
    #[test]
    fn unsafe_reference_cases_preserve_exact_diagnostics_without_rewriting() {
        let mut fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/redundant_regexp_constructor/intentional-safeguards.json"
        )))
        .unwrap();
        for case in fixture.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(
            &RedundantRegexpConstructor,
            &fixture.to_string(),
        );
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &RedundantRegexpConstructor,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/redundant_regexp_constructor/parity.json"
            )),
        );
    }
}
