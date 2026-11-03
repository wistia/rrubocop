//! Style/TrailingCommaInArguments: parenthesized send arguments and punctuation.
use crate::cop::shared::{self, named_kids, node_bytes, push_replace};
use crate::cop::style::trailing_comma_args::{
    begins_its_line, each_elem_own_line, last_item_line_start,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use tree_sitter::{Node, Tree};
pub struct TrailingCommaInArguments;

fn semantic(node: Node<'_>) -> bool {
    !node.is_extra() && !matches!(node.kind(), "comment" | "heredoc_body" | "heredoc_end")
}
fn heredoc(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "heredoc_beginning" => true,
        "pair" | "hash" => named_kids(node)
            .into_iter()
            .filter(|node| semantic(*node))
            .last()
            .is_some_and(|node| heredoc(source, node)),
        "call" => {
            if node
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
            {
                return false;
            }
            shared::argument_nodes(node)
                .into_iter()
                .filter(|node| semantic(*node))
                .last()
                .or_else(|| node.child_by_field_name("receiver"))
                .is_some_and(|node| heredoc(source, node))
        }
        _ => false,
    }
}

impl Cop for TrailingCommaInArguments {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInArguments"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn uses_source_phase(&self) -> bool {
        true
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
        let initial = corrections
            .as_ref()
            .map_or(0, |corrections| corrections.len());
        shared::for_each_descendant(tree.root_node(), |node| {
            if node.kind() == "call" {
                self.check_call(
                    source,
                    node,
                    config,
                    diagnostics,
                    corrections.as_deref_mut(),
                );
            }
        });
        if let Some(corrections) = corrections {
            shared::normalize_corrected_crlf(source, self.name(), corrections, initial);
        }
    }
}
impl TrailingCommaInArguments {
    fn check_call(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if node.has_error()
            || node
                .child_by_field_name("method")
                .is_some_and(|method| matches!(method.kind(), "super" | "yield"))
        {
            return;
        }
        let Some(list) = node.child_by_field_name("arguments") else {
            return;
        };
        if node_bytes(source, list).first() != Some(&b'(')
            || node_bytes(source, list).last() != Some(&b')')
        {
            return;
        }
        let close = list.end_byte() - 1;
        let args = shared::ruby_arguments::from_nodes(
            shared::argument_nodes(node)
                .into_iter()
                .filter(|node| semantic(*node)),
        );
        let Some(last) = args.last() else { return };
        let mut elements = Vec::new();
        for arg in &args {
            if arg.hash
                && !arg.braces
                && source.offset_to_line_col(arg.start()).0
                    != source.offset_to_line_col(arg.end().saturating_sub(1)).0
            {
                elements.extend(
                    arg.parts()
                        .into_iter()
                        .map(|node| (node.start_byte(), node.end_byte())),
                );
            } else {
                elements.push((arg.start(), arg.end()));
            }
        }
        let mut command_owns_tail = false;
        if let Some(first) = args.first().and_then(|arg| arg.node()) {
            let mut end = first.end_byte();
            shared::for_each_descendant(first, |node| {
                if let Some(command) = shared::command_unary_plus_call(source, node) {
                    if command.end > first.end_byte() && command.end <= last.end() {
                        end = end.max(command.end);
                    }
                }
            });
            if end > first.end_byte() {
                command_owns_tail = end == last.end();
                elements.retain(|(_, last)| *last > end);
                elements.insert(0, (first.start_byte(), end));
            }
        }
        let mut unsafe_insertion = command_owns_tail
            || last
                .node()
                .is_some_and(|node| node.kind() == "forward_argument");
        for root in &last.nodes {
            shared::for_each_descendant(*root, |node| {
                if node.kind() == "call"
                    && node.end_byte() == last.end()
                    && node.child_by_field_name("block").is_none()
                {
                    if let Some(arguments) = node.child_by_field_name("arguments") {
                        unsafe_insertion |= node_bytes(source, arguments).first() != Some(&b'(');
                    }
                }
            });
        }
        let style = config.get_str("EnforcedStyleForMultiline", "no_comma");
        let multiline = source.offset_to_line_col(node.start_byte()).0
            != source.offset_to_line_col(close).0
            && !(elements.len() == 1 && !begins_its_line(source, close));
        let selector = node
            .child_by_field_name("method")
            .map_or(node.start_byte(), |node| node.start_byte());
        let last_line = source.offset_to_line_col(last.end().saturating_sub(1)).0;
        let same_line = source.offset_to_line_col(selector).0 == last_line
            && source.offset_to_line_col(close).0 == last_line;
        let want = multiline
            && match style {
                "comma" => each_elem_own_line(source, &elements, close),
                "consistent_comma" => !same_line,
                _ => false,
            };
        let any_heredoc = args
            .iter()
            .any(|arg| arg.nodes.last().is_some_and(|node| heredoc(source, *node)));
        let after = &source.as_bytes()[last.end()..close];
        let comma = after
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .filter(|offset| {
                after[*offset] == b',' && (!any_heredoc || !after[..*offset].contains(&b'\n'))
            })
            .map(|offset| last.end() + offset);
        let (start, end, message, edit_start, edit_end, replacement) = match (comma, want) {
            (Some(comma), false) => {
                let suffix = match style {
                    "comma" => ", unless each item is on its own line",
                    "consistent_comma" => ", unless items are split onto multiple lines",
                    _ => "",
                };
                (
                    comma,
                    comma + 1,
                    format!("Avoid comma after the last parameter of a method call{suffix}."),
                    comma,
                    comma + 1,
                    "",
                )
            }
            (None, true)
                if command_owns_tail
                    || !last
                        .node()
                        .is_some_and(|node| node.kind() == "block_argument") =>
            {
                (
                    last_item_line_start(source, last.start(), last.end()),
                    last.end(),
                    "Put a comma after the last parameter of a multiline method call.".into(),
                    last.end(),
                    last.end(),
                    ",",
                )
            }
            _ => return,
        };
        let (line, column) = source.offset_to_line_col(start);
        let mut diagnostic = self.diagnostic(source, line, column, message);
        diagnostic.highlight_length = String::from_utf8_lossy(&source.as_bytes()[start..end])
            .chars()
            .count();
        if replacement != "," || !unsafe_insertion {
            diagnostic.corrected = push_replace(
                &mut corrections,
                edit_start,
                edit_end,
                replacement,
                self.name(),
            );
        }
        diagnostics.push(diagnostic);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TrailingCommaInArguments,
        "cops/style/trailing_comma_in_arguments"
    );

    #[test]
    fn original_spec_states() {
        crate::testutil::assert_original_spec_parity(
            &TrailingCommaInArguments,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_arguments/original-parity.json"
            )),
        );
    }

    #[test]
    fn independent_original_states() {
        crate::testutil::assert_original_spec_parity(
            &TrailingCommaInArguments,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_arguments/independent-parity.json"
            )),
        );
    }

    #[test]
    fn unsafe_original_insertions_are_declined() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/trailing_comma_in_arguments/safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(
            &TrailingCommaInArguments,
            &serde_json::to_string(&cases).unwrap(),
        );
    }

    #[test]
    fn guard_controls_and_final_native_states() {
        for fixture in [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_arguments/guard-controls.json"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_arguments/final-states.json"
            )),
        ] {
            let mut cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
            for case in cases.as_array_mut().unwrap() {
                if case.get("rust_corrected").is_some() {
                    case["ruby_corrected"] = case["rust_corrected"].clone();
                }
            }
            crate::testutil::assert_original_spec_parity(
                &TrailingCommaInArguments,
                &serde_json::to_string(&cases).unwrap(),
            );
        }
    }

    #[test]
    fn covered_sources_have_clean_native_trees() {
        let mut errors = Vec::new();
        for fixture in [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_arguments/original-parity.json"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_arguments/independent-parity.json"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_arguments/safeguards.json"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_arguments/guard-controls.json"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_arguments/final-states.json"
            )),
        ] {
            let cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
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
        }
        assert!(
            errors.is_empty(),
            "NATIVE_ERROR_COVERAGE {}",
            serde_json::to_string(&errors).unwrap()
        );
    }

    fn comma_cfg() -> CopConfig {
        let mut c = CopConfig::default();
        c.options.insert(
            "EnforcedStyleForMultiline".into(),
            serde_yml::Value::String("comma".into()),
        );
        c
    }

    #[test]
    fn comma_style_single_line_kwargs_ok() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &TrailingCommaInArguments,
            b"foo(\n  a: 1, b: 2,\n)\n",
            comma_cfg(),
        );
    }

    #[test]
    fn comma_style_shared_line_args_no_trailing_ok() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &TrailingCommaInArguments,
            b"foo(\n  1, 2, 3\n)\n",
            comma_cfg(),
        );
    }

    #[test]
    fn comma_style_heredoc_kwargs_trailing_ok() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &TrailingCommaInArguments,
            b"foo(\n  msg: <<~MESSAGE,\n    body\n  MESSAGE\n  level: :error,\n)\n",
            comma_cfg(),
        );
    }

    #[test]
    fn put_comma_highlights_last_arg_start() {
        let mut c = CopConfig::default();
        c.options.insert(
            "EnforcedStyleForMultiline".into(),
            serde_yml::Value::String("consistent_comma".into()),
        );
        let diags = crate::testutil::run_cop_full_with_config(
            &TrailingCommaInArguments,
            b"qux(\n  first,\n  last\n)\n",
            c,
        );
        assert_eq!(diags.len(), 1, "got: {diags:?}");
        assert_eq!(diags[0].location.column, 2); // 0-based: start of `last`
        assert!(diags[0].message.starts_with("Put a comma"));
    }
}
