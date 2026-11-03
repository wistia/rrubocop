//! Style/TrailingCommaInBlockArgs: original parameter roles and pipe tokens.
use crate::cop::shared::{self, named_kids, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use std::cmp::Reverse;
use std::collections::HashSet;
use tree_sitter::{Node, Tree};

pub struct TrailingCommaInBlockArgs;
struct Block<'a> {
    source: Node<'a>,
    parameters: Option<Node<'a>>,
    lambda_literal: bool,
}
fn shadow(node: Node<'_>) -> bool {
    let Some(parent) = node.parent().filter(|p| p.kind() == "block_parameters") else {
        return false;
    };
    let mut cursor = parent.walk();
    parent
        .children_by_field_name("locals", &mut cursor)
        .any(|local| local == node)
}
fn argument_count(parameters: Node<'_>) -> usize {
    let mut count = 0;
    shared::for_each_descendant(parameters, |node| {
        count += usize::from(match node.kind() {
            "optional_parameter" => true,
            "keyword_parameter" => node.child_by_field_name("value").is_some(),
            "identifier" => {
                node.parent().is_some_and(|p| {
                    matches!(
                        p.kind(),
                        "block_parameters"
                            | "method_parameters"
                            | "lambda_parameters"
                            | "destructured_parameter"
                    )
                }) && !shadow(node)
            }
            _ => false,
        });
    });
    count
}
fn top_level_parameter_count(parameters: Node<'_>) -> usize {
    named_kids(parameters)
        .into_iter()
        .filter(|n| !n.is_extra() && n.kind() != "comment" && !shadow(*n))
        .count()
}
fn blocks<'a>(tree: &'a Tree, source: &SourceFile) -> Vec<Block<'a>> {
    let mut result = Vec::new();
    let mut pending = vec![tree.root_node()];
    while let Some(node) = pending.pop() {
        if matches!(node.kind(), "block" | "do_block") && !node.has_error() {
            let parent = node.parent();
            let lambda = parent.is_some_and(|p| p.kind() == "lambda");
            let parameters = node.child_by_field_name("parameters").or_else(|| {
                parent
                    .filter(|_| lambda)
                    .and_then(|p| p.child_by_field_name("parameters"))
            });
            let numbered = parameters.is_none()
                && node
                    .child_by_field_name("body")
                    .is_some_and(|body| shared::numbered_parameter_arity(source, body) > 0);
            if !numbered {
                let expression = parent
                    .filter(|p| {
                        p.kind() == "lambda"
                            || (p.kind() == "call" && p.child_by_field_name("block") == Some(node))
                    })
                    .unwrap_or(node);
                result.push(Block {
                    source: expression,
                    parameters,
                    lambda_literal: lambda,
                });
            }
        }
        pending.extend(named_kids(node).into_iter().rev());
    }
    result.sort_by_key(|b| (b.source.start_byte(), Reverse(b.source.end_byte())));
    result
}
fn pipe(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(node.kind(), "|" | "operator") && node_bytes(source, node) == b"|"
}
fn tokens<'a>(source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    shared::ruby_token_nodes(source, node)
        .into_iter()
        .filter(|n| !n.is_extra() || n.kind() == "comment")
        .collect()
}
fn comma<'a>(source: &SourceFile, block: Node<'a>) -> Option<Node<'a>> {
    let tokens = tokens(source, block);
    let opening = tokens.iter().position(|node| pipe(source, *node))?;
    let closing = tokens
        .iter()
        .enumerate()
        .skip(opening + 1)
        .find_map(|(i, n)| pipe(source, *n).then_some(i))?;
    (closing > opening + 1)
        .then(|| tokens[closing - 1])
        .filter(|node| node.kind() == ",")
}
fn preserves_binding(source: &SourceFile, node: Node<'_>, blocks: &[Block<'_>]) -> bool {
    blocks
        .iter()
        .filter_map(|block| block.parameters)
        .any(|parameters| {
            if node.parent() != Some(parameters) || top_level_parameter_count(parameters) < 2 {
                return false;
            }
            let Some(after) = source
                .as_bytes()
                .get(node.end_byte()..parameters.end_byte())
            else {
                return false;
            };
            after.last() == Some(&b'|')
                && after[..after.len() - 1].iter().all(u8::is_ascii_whitespace)
        })
}
impl Cop for TrailingCommaInBlockArgs {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInBlockArgs"
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
        let blocks = blocks(tree, source);
        let initial = corrections.as_ref().map_or(0, |edits| edits.len());
        let mut seen = HashSet::new();
        for block in &blocks {
            if block.lambda_literal || !block.parameters.is_some_and(|p| argument_count(p) > 1) {
                continue;
            }
            let Some(comma) = comma(source, block.source) else {
                continue;
            };
            if !seen.insert(comma.start_byte()) {
                continue;
            }
            let (line, column) = source.offset_to_line_col(comma.start_byte());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                "Useless trailing comma present in block arguments.".into(),
            );
            diagnostic.highlight_length = 1;
            diagnostic.correctable =
                preserves_binding(source, comma, &blocks) && !config.autocorrect_disabled();
            if diagnostic.correctable {
                diagnostic.corrected = push_replace(
                    &mut corrections,
                    comma.start_byte(),
                    comma.end_byte(),
                    "",
                    self.name(),
                );
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
            &TrailingCommaInBlockArgs,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_block_args/original-parity.json"
            )),
        );
    }
    #[test]
    fn independent_original_states() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/trailing_comma_in_block_args/original-independent.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            if case.get("rust_corrected").is_some() {
                case["ruby_corrected"] = case["rust_corrected"].clone();
            }
        }
        crate::testutil::assert_original_spec_parity(
            &TrailingCommaInBlockArgs,
            &serde_json::to_string(&cases).unwrap(),
        );
    }
    #[test]
    fn nullable_safety_override_preserves_safe_mode_source() {
        use crate::cli::AutocorrectMode;
        use crate::config::{CopFilterSet, ResolvedConfig};
        use crate::cop::{CopConfig, CopRegistry, EnabledState};
        let registry = CopRegistry::default_registry();
        let mut config = ResolvedConfig::empty();
        config.cop_configs.insert(
            "Style/TrailingCommaInBlockArgs".into(),
            CopConfig {
                enabled: EnabledState::True,
                options: serde_json::from_value(
                    serde_json::json!({"Safe":true,"SafeAutoCorrect":null}),
                )
                .unwrap(),
                ..CopConfig::default()
            },
        );
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            let source = b"items { |a, b,| work }\n";
            let mut bytes = source.to_vec();
            let diagnostics = crate::linter::lint_bytes_autocorrect(
                std::path::Path::new("probe.rb"),
                &mut bytes,
                &config,
                &registry,
                &CopFilterSet::build(&config, &registry),
                Some(&["Style/TrailingCommaInBlockArgs".into()]),
                &[],
                mode,
                false,
            )
            .unwrap();
            assert_eq!(
                bytes,
                if mode == AutocorrectMode::All {
                    b"items { |a, b| work }\n".as_slice()
                } else {
                    source.as_slice()
                }
            );
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].corrected, mode == AutocorrectMode::All);
        }
    }
    #[test]
    fn original_roles() {
        let rows: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/trailing_comma_in_block_args/original-roles.json"
        )))
        .unwrap();
        for row in rows.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                row["original"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(
                !tree.root_node().has_error(),
                "{} {}",
                row["id"],
                tree.root_node().to_sexp()
            );
            let actual:Vec<_>=blocks(&tree,&source).into_iter().map(|block| {
                let pipes:Vec<_>=tokens(&source,block.source).into_iter().filter(|n|pipe(&source,*n)).take(2).map(|n|{let (l,c)=source.offset_to_line_col(n.start_byte());serde_json::json!([l,c])}).collect();
                let comma=comma(&source,block.source).map(|n| {let(l,c)=source.offset_to_line_col(n.start_byte());serde_json::json!([l,c])});
                serde_json::json!({"source":String::from_utf8_lossy(node_bytes(&source,block.source)).replace("\r\n","\n"),"count":block.parameters.map_or(0,argument_count),"top":block.parameters.map_or(0,top_level_parameter_count),"lambda":block.lambda_literal,"pipes":pipes,"comma":comma})
            }).collect();
            let expected:Vec<_>=row["blocks"].as_array().unwrap().iter().map(|block|serde_json::json!({"source":block["source"],"count":block["descendant_argument_count"],"top":block["parameter_types"].as_array().unwrap().iter().filter(|t|t.as_str()!=Some("shadowarg")).count(),"lambda":block["lambda_literal"],"pipes":block["first_pipes"],"comma":if block["last_token_between_pipes"]["type"]=="tCOMMA" {serde_json::json!([block["last_token_between_pipes"]["line"],block["last_token_between_pipes"]["column"]])}else{serde_json::Value::Null}})).collect();
            assert_eq!(
                actual,
                expected,
                "{} {}",
                row["id"],
                tree.root_node().to_sexp()
            );
        }
    }
}
