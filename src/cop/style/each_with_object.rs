//! Style/EachWithObject: original inject/reduce object and returned-local predicate.
use crate::cop::shared::{
    call_method_name, method_node, node_bytes, push_replace, semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct EachWithObject;
mod pattern;
mod safety;
use pattern::{arity, assigned, basic_literal, children, numbered, parameter_name, returned};
use safety::{
    different_next, nested_numbered, pattern_assignment, reflective, shadow_parameter,
    shorthand_numbered,
};

fn numbered_reads<'n>(source: &SourceFile, node: Node<'n>, result: &mut Vec<Node<'n>>) {
    if node.kind() == "identifier"
        && matches!(numbered(source, node), Some(1 | 2))
        && crate::cop::ported::ruby_local_identifier(source, node)
    {
        result.push(node);
    }
    for child in children(node) {
        numbered_reads(source, child, result);
    }
}
fn return_range(source: &SourceFile, node: Node<'_>) -> (usize, usize) {
    let bytes = source.as_bytes();
    let start = bytes[..node.start_byte()]
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |n| n + 1);
    let end = bytes[node.end_byte()..]
        .iter()
        .position(|b| *b == b'\n')
        .map_or(bytes.len(), |n| node.end_byte() + n + 1);
    let line = &bytes[start..end];
    let first = line
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(line.len());
    let last = line
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(first, |n| n + 1);
    if &line[first..last] == node_bytes(source, node) {
        (start, end)
    } else {
        (node.start_byte(), node.end_byte())
    }
}
impl Cop for EachWithObject {
    fn name(&self) -> &'static str {
        "Style/EachWithObject"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"inject", b"reduce"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) =
            call_method_name(source, node).filter(|m| matches!(*m, b"inject" | b"reduce"))
        else {
            return;
        };
        let args = semantic_argument_nodes(node);
        if args.is_empty()
            || args.len() != 1
                && !args
                    .iter()
                    .all(|n| matches!(n.kind(), "pair" | "hash_splat_argument"))
            || args.iter().any(|n| n.kind() == "block_argument")
        {
            return;
        }
        if args.len() == 1 && basic_literal(source, args[0]) {
            return;
        }
        let Some(block) = node.child_by_field_name("block") else {
            return;
        };
        let Some(body) = block.child_by_field_name("body") else {
            return;
        };
        let Some(value) =
            returned(body).filter(|n| crate::cop::ported::ruby_local_identifier(source, *n))
        else {
            return;
        };
        let params = block.child_by_field_name("parameters");
        let explicit = params.map(children);
        if let Some(params) = &explicit {
            let Some(first) = params.first().copied().and_then(parameter_name) else {
                return;
            };
            if node_bytes(source, first) != node_bytes(source, value)
                || assigned(source, body, node_bytes(source, first))
            {
                return;
            }
        } else if config.get_f64("TargetRubyVersion", 2.7) < 2.7
            || arity(source, body) != 2
            || node_bytes(source, value) != b"_1"
        {
            return;
        }
        let Some(selector) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `each_with_object` instead of `{}`.",
                String::from_utf8_lossy(method)
            ),
        );
        diagnostic.highlight_length = method.len();
        let unsafe_args = args.iter().any(|n| {
            matches!(
                n.kind(),
                "splat_argument" | "hash_splat_argument" | "argument_forwarding"
            )
        });
        let unsafe_params = explicit.as_ref().is_some_and(|params| {
            params.len() < 2
                || matches!(
                    params[0].kind(),
                    "splat_parameter"
                        | "hash_splat_parameter"
                        | "block_parameter"
                        | "keyword_parameter"
                )
                || params.iter().take(2).any(|n| {
                    shadow_parameter(*n)
                        || matches!(
                            n.kind(),
                            "keyword_parameter" | "hash_splat_parameter" | "block_parameter"
                        )
                })
        });
        let unsafe_body = reflective(source, body)
            || different_next(source, body, node_bytes(source, value))
            || pattern_assignment(source, body, node_bytes(source, value))
            || explicit.is_none()
                && (nested_numbered(source, body, false) || shorthand_numbered(source, body));
        if unsafe_args || unsafe_params || unsafe_body {
            diagnostic.correctable = false;
            diagnostics.push(diagnostic);
            return;
        }
        if let Some(params) = explicit {
            let (Ok(first), Ok(second)) = (
                std::str::from_utf8(node_bytes(source, params[0])),
                std::str::from_utf8(node_bytes(source, params[1])),
            ) else {
                diagnostic.correctable = false;
                diagnostics.push(diagnostic);
                return;
            };
            diagnostic.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                selector.end_byte(),
                "each_with_object",
                self.name(),
            );
            push_replace(
                &mut corrections,
                params[0].start_byte(),
                params[0].end_byte(),
                second,
                self.name(),
            );
            push_replace(
                &mut corrections,
                params[1].start_byte(),
                params[1].end_byte(),
                first,
                self.name(),
            );
            let (start, end) = return_range(source, value);
            push_replace(&mut corrections, start, end, "", self.name());
        } else {
            diagnostic.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                selector.end_byte(),
                "each_with_object",
                self.name(),
            );
            let mut reads = Vec::new();
            numbered_reads(source, body, &mut reads);
            for read in reads {
                push_replace(
                    &mut corrections,
                    read.start_byte(),
                    read.end_byte(),
                    if node_bytes(source, read) == b"_1" {
                        "_2"
                    } else {
                        "_1"
                    },
                    self.name(),
                );
            }
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
            "/tests/fixtures/cops/style/each_with_object/intentional-safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&EachWithObject, &cases.to_string());
    }
    #[test]
    fn missing_second_parameter_reports_without_original_nil_dereference() {
        let result =
            crate::testutil::run_cop_full(&EachWithObject, b"items.inject({}) { |memo| memo }\n");
        assert_eq!(result.len(), 1);
        assert!(!result[0].correctable);
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &EachWithObject,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/each_with_object/parity.json"
            )),
        );
    }
}
