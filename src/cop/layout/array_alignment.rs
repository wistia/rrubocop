//! Layout/ArrayAlignment: original display geometry and nested-element correction.
mod display_width;
use crate::cop::shared::{named_kids, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use tree_sitter::{Node, Tree};

pub struct ArrayAlignment;
#[derive(Clone, Copy)]
struct Item {
    start: usize,
    end: usize,
}
struct Interior {
    node: Item,
    inside: Item,
}
fn items(node: Node<'_>) -> Vec<Item> {
    let mut result: Vec<Item> = Vec::new();
    let mut in_hash = false;
    for child in named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
    {
        let hash = matches!(child.kind(), "pair" | "hash_splat_argument");
        if hash && in_hash {
            result.last_mut().unwrap().end = child.end_byte();
        } else {
            result.push(Item {
                start: child.start_byte(),
                end: child.end_byte(),
            });
        }
        in_hash = hash;
    }
    result
}
fn line(source: &SourceFile, start: usize) -> usize {
    source.offset_to_line_col(start).0
}
fn begins_line(source: &SourceFile, start: usize) -> bool {
    let line_start = source.line_start(line(source, start)).unwrap_or(0);
    source.as_bytes()[line_start..start]
        .iter()
        .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\x0b' | b'\x0c'))
}
fn display_column(source: &SourceFile, start: usize) -> usize {
    let line_start = source.line_start(line(source, start)).unwrap_or(0);
    display_width::of(&String::from_utf8_lossy(
        &source.as_bytes()[line_start..start],
    ))
}
fn parallel_assignment(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        parent.kind() == "assignment"
            && parent.child_by_field_name("left").is_some_and(|left| {
                matches!(
                    left.kind(),
                    "left_assignment_list" | "destructured_left_assignment"
                )
            })
    })
}
fn base_column(source: &SourceFile, node: Node<'_>, first: Item, config: &CopConfig) -> usize {
    if config.get_str("EnforcedStyle", "with_first_element") != "with_fixed_indentation" {
        return display_column(source, first.start);
    }
    let target = if matches!(node.kind(), "right_assignment_list" | "exceptions") {
        node.parent().unwrap_or(node)
    } else {
        node
    };
    let start = source
        .line_start(line(source, target.start_byte()))
        .unwrap_or(0);
    let prefix = &source.as_bytes()[start..target.start_byte()];
    prefix
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\x0b' | b'\x0c'))
        .unwrap_or(prefix.len())
        + config.get_usize("IndentationWidth", 2)
}
fn shift(
    source: &SourceFile,
    item: Item,
    delta: isize,
    taboo: &[Interior],
    comments: &[(usize, usize)],
    corrections: &mut Vec<Correction>,
) -> bool {
    if comments
        .iter()
        .any(|(start, end)| *start >= item.start && *end <= item.end)
    {
        return false;
    }
    let initial = corrections.len();
    let mut start = item.start;
    while start < item.end {
        let end = if delta > 0 {
            start
        } else if source.as_bytes()[start] == b' ' {
            start.saturating_add((-delta) as usize)
        } else {
            start
        };
        let left = if delta < 0 && source.as_bytes()[start] != b' ' {
            start.checked_sub((-delta) as usize)
        } else {
            Some(start)
        };
        if let Some(left) = left {
            if end <= source.as_bytes().len()
                && !taboo.iter().any(|literal| {
                    literal.node.start >= item.start
                        && literal.node.end <= item.end
                        && left >= literal.inside.start
                        && end <= literal.inside.end
                })
            {
                let allowed = if delta > 0 {
                    !matches!(source.as_bytes().get(start), None | Some(b'\n' | b'\r'))
                } else {
                    left < end
                        && source.as_bytes()[left..end]
                            .iter()
                            .all(|b| matches!(b, b' ' | b'\t'))
                };
                if allowed {
                    corrections.push(Correction {
                        start: left,
                        end,
                        replacement: if delta > 0 {
                            " ".repeat(delta as usize)
                        } else {
                            String::new()
                        },
                        cop_name: "Layout/ArrayAlignment",
                        cop_index: 0,
                        owner: Some(item.start),
                    });
                }
            }
        }
        let Some(newline) = source.as_bytes()[start..item.end]
            .iter()
            .position(|b| *b == b'\n')
        else {
            break;
        };
        start += newline + 1;
    }
    corrections.len() > initial
}
impl Cop for ArrayAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArrayAlignment"
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
        let mut pending = vec![tree.root_node()];
        let mut arrays = Vec::new();
        let mut taboo = Vec::new();
        let mut comments = Vec::new();
        while let Some(node) = pending.pop() {
            if matches!(
                node.kind(),
                "array" | "right_assignment_list" | "string_array" | "symbol_array" | "exceptions"
            ) {
                arrays.push(node);
            }
            if matches!(
                node.kind(),
                "string" | "subshell" | "regex" | "delimited_symbol"
            ) && node.child_count() >= 2
            {
                taboo.push(Interior {
                    node: Item {
                        start: node.start_byte(),
                        end: node.end_byte(),
                    },
                    inside: Item {
                        start: node.child(0).unwrap().end_byte(),
                        end: node
                            .child((node.child_count() - 1) as u32)
                            .unwrap()
                            .start_byte(),
                    },
                });
            } else if matches!(node.kind(), "bare_string" | "bare_symbol")
                && node_bytes(source, node).contains(&b'\n')
            {
                taboo.push(Interior {
                    node: Item {
                        start: node.start_byte(),
                        end: node.end_byte(),
                    },
                    inside: Item {
                        start: node.start_byte() + 1,
                        end: node.end_byte(),
                    },
                });
            } else if node.kind() == "heredoc_beginning" {
                if let Some((start, end)) = crate::cop::ported::heredoc_span(source, node) {
                    let end = source.as_bytes()[end..]
                        .iter()
                        .position(|b| *b == b'\n')
                        .map_or(source.as_bytes().len(), |offset| end + offset);
                    taboo.push(Interior {
                        node: Item {
                            start: node.start_byte(),
                            end: node.end_byte(),
                        },
                        inside: Item { start, end },
                    });
                }
            } else if node.kind() == "comment" && node_bytes(source, node).starts_with(b"=begin") {
                comments.push((node.start_byte(), node.end_byte()));
            }
            pending.extend(named_kids(node).into_iter().rev());
        }
        let initial = corrections.as_ref().map_or(0, |v| v.len());
        let mut offenses: Vec<Item> = Vec::new();
        for node in arrays {
            if parallel_assignment(node) {
                continue;
            }
            let elements = items(node);
            if elements.len() < 2 {
                continue;
            }
            let base = base_column(source, node, elements[0], config);
            let mut previous = 0;
            for item in elements {
                let current = line(source, item.start);
                let check = current > previous && begins_line(source, item.start);
                previous = current;
                let delta = base as isize - display_column(source, item.start) as isize;
                if !check || delta == 0 {
                    continue;
                }
                let (line, column) = source.offset_to_line_col(item.start);
                let message = if config.get_str("EnforcedStyle", "with_first_element")
                    == "with_fixed_indentation"
                {
                    "Use one level of indentation for elements following the first line of a multi-line array."
                } else {
                    "Align the elements of an array literal if they span more than one line."
                };
                let mut diagnostic = self.diagnostic(source, line, column, message.into());
                diagnostic.highlight_length =
                    String::from_utf8_lossy(&source.as_bytes()[item.start..item.end])
                        .replace("\r\n", "\n")
                        .chars()
                        .count();
                if !offenses
                    .iter()
                    .any(|outer| item.start >= outer.start && item.end <= outer.end)
                {
                    if let Some(corrections) = corrections.as_deref_mut() {
                        diagnostic.corrected =
                            shift(source, item, delta, &taboo, &comments, corrections);
                    }
                }
                offenses.push(item);
                diagnostics.push(diagnostic);
            }
        }
        if let Some(corrections) = corrections {
            if corrections.len() > initial {
                let owner = corrections[initial].owner;
                let ranges: Vec<_> = corrections[initial..]
                    .iter()
                    .map(|c| (c.start, c.end))
                    .collect();
                for (start, bytes) in source.as_bytes().windows(2).enumerate() {
                    if bytes == b"\r\n" && !ranges.iter().any(|(a, b)| start >= *a && start < *b) {
                        corrections.push(Correction {
                            start,
                            end: start + 1,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                            owner,
                        });
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_pipeline_preserves_multiline_correction_ownership() {
        use crate::cli::AutocorrectMode;
        use crate::config::{CopFilterSet, ResolvedConfig};
        use crate::cop::{CopRegistry, EnabledState};
        for (before, after) in [
            ("a = [\r\n  one,\r\n    two]\r\n", "a = [\n  one,\n  two]\n"),
            (
                "[one,\r\n    [two,\r\n       three]]\r\n",
                "[one,\n [two,\n  three]]\n",
            ),
        ] {
            for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
                let mut config = ResolvedConfig::empty();
                config.cop_configs.insert(
                    ArrayAlignment.name().into(),
                    CopConfig {
                        enabled: EnabledState::True,
                        options: [
                            ("EnforcedStyle".into(), "with_first_element".into()),
                            ("__RubocopVersion".into(), "1.65.1".into()),
                        ]
                        .into(),
                        ..CopConfig::default()
                    },
                );
                let registry = CopRegistry::default_registry();
                let filters = CopFilterSet::build(&config, &registry);
                let only = [ArrayAlignment.name().into()];
                let mut bytes = before.as_bytes().to_vec();
                crate::linter::lint_bytes_autocorrect(
                    std::path::Path::new("probe.rb"),
                    &mut bytes,
                    &config,
                    &registry,
                    &filters,
                    Some(&only),
                    &[],
                    mode,
                    false,
                )
                .unwrap();
                assert_eq!(bytes, after.as_bytes());
                let remaining = crate::linter::lint_bytes_autocorrect(
                    std::path::Path::new("probe.rb"),
                    &mut bytes,
                    &config,
                    &registry,
                    &filters,
                    Some(&only),
                    &[],
                    mode,
                    false,
                )
                .unwrap();
                assert!(remaining.is_empty());
                assert_eq!(bytes, after.as_bytes());
            }
        }
    }
    #[test]
    fn independent_original_states() {
        crate::testutil::assert_original_spec_parity(
            &ArrayAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/array_alignment/independent-parity.json"
            )),
        );
    }
    #[test]
    fn original_literal_changes_are_not_reproduced() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/array_alignment/safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(
            &ArrayAlignment,
            &serde_json::to_string(&cases).unwrap(),
        );
    }
    #[test]
    fn original_spec_states() {
        crate::testutil::assert_original_spec_parity(
            &ArrayAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/array_alignment/original-parity.json"
            )),
        );
    }
}
