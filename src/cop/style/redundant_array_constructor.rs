//! Style/RedundantArrayConstructor.
use crate::cop::shared::{
    call_method_name, call_receiver, method_node, node_bytes, push_replace, semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct RedundantArrayConstructor;

fn root_array(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(node_bytes(source, node), b"Array" | b"::Array")
}

impl Cop for RedundantArrayConstructor {
    fn name(&self) -> &'static str {
        "Style/RedundantArrayConstructor"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "element_reference"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let (start, end, replacement_start, replacement_end) = if node.kind() == "element_reference"
        {
            let Some(receiver) = node
                .child_by_field_name("object")
                .filter(|receiver| root_array(source, *receiver))
            else {
                return;
            };
            (
                receiver.start_byte(),
                receiver.end_byte(),
                receiver.end_byte(),
                node.end_byte(),
            )
        } else {
            if node
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
            {
                return;
            }
            let Some(method) = method_node(node) else {
                return;
            };
            let name = call_method_name(source, node);
            let receiver = call_receiver(node);
            if name == Some(b"[]") && receiver.is_some_and(|receiver| root_array(source, receiver))
            {
                let receiver = receiver.unwrap();
                (
                    receiver.start_byte(),
                    receiver.end_byte(),
                    method.start_byte(),
                    node.end_byte(),
                )
            } else {
                if !((name == Some(b"new")
                    && receiver.is_some_and(|receiver| root_array(source, receiver)))
                    || (name == Some(b"Array") && receiver.is_none()))
                {
                    return;
                }
                let args = semantic_argument_nodes(node);
                if args.len() != 1
                    || !matches!(args[0].kind(), "array" | "string_array" | "symbol_array")
                {
                    return;
                }
                (
                    receiver.map_or(method.start_byte(), |receiver| receiver.start_byte()),
                    method.end_byte(),
                    args[0].start_byte(),
                    args[0].end_byte(),
                )
            }
        };
        let (line, column) = source.offset_to_line_col(start);
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Remove the redundant `Array` constructor.".into(),
        );
        diagnostic.highlight_length = String::from_utf8_lossy(&source.as_bytes()[start..end])
            .chars()
            .count();
        let drops_comment = crate::cop::shared::collect_comments(node)
            .iter()
            .any(|comment| {
                comment.start_byte() < replacement_start || comment.end_byte() > replacement_end
            });
        let explicit_bracket_call = node.kind() == "call"
            && call_method_name(source, node) == Some(b"[]")
            && method_node(node).is_some_and(|method| method.end_byte() < node.end_byte());
        if node.child_by_field_name("block").is_some() || drops_comment || explicit_bracket_call {
            diagnostic.correctable = false;
        } else if let Ok(replacement) =
            std::str::from_utf8(&source.as_bytes()[replacement_start..replacement_end])
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
    fn does_not_copy_invalid_ruby_corrections_or_delete_comments() {
        for input in [
            b"Array.new([1]) { nil }".as_slice(),
            b"Array.[](1)",
            b"Array.new( # retain\n [1])",
        ] {
            let diagnostics = crate::testutil::run_cop_full(&RedundantArrayConstructor, input);
            assert_eq!(diagnostics.len(), 1);
            assert!(!diagnostics[0].correctable);
        }
    }
    #[test]
    fn preserves_non_utf8_literal_bytes() {
        let diagnostics = crate::testutil::run_cop_full(
            &RedundantArrayConstructor,
            b"# coding: ASCII-8BIT\nArray.new([\"\xff\"])\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &RedundantArrayConstructor,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/redundant_array_constructor/parity.json"
            )),
        );
    }
}
