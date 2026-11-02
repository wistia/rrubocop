//! Layout/HashAlignment: original style selection and guarded per-element correction.
mod correction;
mod geometry;
mod syntax;
use crate::cop::shared::named_kids;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use correction::edits;
use geometry::{Delta, Style, deltas, last_line, line, same_line, styles};
use syntax::{group, skip};
use tree_sitter::{Node, Tree};
pub struct HashAlignment;

impl Cop for HashAlignment {
    fn name(&self) -> &'static str {
        "Layout/HashAlignment"
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
        let initial = corrections.as_ref().map_or(0, |v| v.len());
        let mut pending = vec![tree.root_node()];
        let mut widths = [0, 0];
        while let Some(node) = pending.pop() {
            if matches!(
                node.kind(),
                "hash" | "argument_list" | "command_argument_list" | "array" | "element_reference"
            ) {
                self.check_hash(
                    source,
                    node,
                    config,
                    diagnostics,
                    corrections.as_deref_mut(),
                    &mut widths,
                );
            }
            pending.extend(named_kids(node).into_iter().rev());
        }
        if let Some(corrections) = corrections {
            if corrections.len() > initial {
                let owner = corrections[initial].owner;
                let ranges: Vec<_> = corrections[initial..]
                    .iter()
                    .map(|c| (c.start, c.end))
                    .collect();
                for (start, bytes) in source.as_bytes().windows(2).enumerate() {
                    if bytes == b"\r\n"
                        && !ranges
                            .iter()
                            .any(|(left, right)| start >= *left && start < *right)
                    {
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
impl HashAlignment {
    fn check_hash(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
        widths: &mut [i64; 2],
    ) {
        let Some(group) = group(node) else {
            return;
        };
        let Some(first) = group.elements.iter().copied().find(|e| !e.splat()) else {
            return;
        };
        if (if group.braced {
            line(source, node) == last_line(source, node)
        } else {
            line(source, group.elements[0].node)
                == last_line(source, group.elements.last().unwrap().node)
        }) || skip(source, &group, first, config)
        {
            return;
        }
        let pairs: Vec<_> = group
            .elements
            .iter()
            .filter(|e| !e.splat())
            .copied()
            .collect();
        let checkable = !pairs.windows(2).any(|p| same_line(source, p[0], p[1]))
            && pairs.iter().all(|p| p.rocket == first.rocket);
        let rocket = styles(config, true);
        let colon = styles(config, false);
        if [&rocket, &colon]
            .into_iter()
            .any(|list| !list.iter().any(|style| *style == Style::Key || checkable))
        {
            return;
        }
        let max_width = pairs
            .iter()
            .map(|p| {
                String::from_utf8_lossy(&source.as_bytes()[p.key.byte_range()])
                    .replace("\r\n", "\n")
                    .chars()
                    .count() as i64
            })
            .max()
            .unwrap_or(0);
        let initial_styles = if first.rocket { &rocket } else { &colon };
        if initial_styles.contains(&Style::Table) {
            widths[usize::from(first.rocket)] = max_width;
        }
        let mut offenses: Vec<(Style, Vec<usize>)> = Vec::new();
        let mut changes: Vec<(Style, usize, Delta)> = Vec::new();
        let mut record = |style, index, delta: Delta| {
            let group_index = offenses
                .iter()
                .position(|(s, _)| *s == style)
                .unwrap_or_else(|| {
                    offenses.push((style, Vec::new()));
                    offenses.len() - 1
                });
            if delta.bad() {
                offenses[group_index].1.push(index);
                changes.retain(|(s, i, _)| *s != style || *i != index);
                changes.push((style, index, delta));
            }
        };
        let first_index = group
            .elements
            .iter()
            .position(|e| e.node.id() == first.node.id())
            .unwrap();
        for style in initial_styles {
            record(
                *style,
                first_index,
                deltas(source, first, first, *style, max_width, true),
            );
        }
        for (index, current) in group.elements.iter().copied().enumerate() {
            let list = if current.splat() {
                &[Style::Splat][..]
            } else if current.rocket {
                &rocket
            } else {
                &colon
            };
            for style in list {
                let width = widths[usize::from(current.rocket)];
                record(
                    *style,
                    index,
                    deltas(source, first, current, *style, width, false),
                );
            }
        }
        let mut selected = Vec::new();
        if let Some(index) = offenses.iter().position(|(s, _)| *s == Style::Splat) {
            selected.push(offenses.remove(index));
        }
        if let Some(group) = offenses.into_iter().min_by_key(|(_, v)| v.len()) {
            selected.push(group);
        }
        let mut seen = Vec::new();
        for (style, indices) in selected {
            for index in indices {
                if seen.contains(&index) {
                    continue;
                }
                seen.push(index);
                let element = group.elements[index];
                let (line, column) = source.offset_to_line_col(element.node.start_byte());
                let mut diagnostic = self.diagnostic(source, line, column, style.message().into());
                diagnostic.highlight_length =
                    String::from_utf8_lossy(&source.as_bytes()[element.node.byte_range()])
                        .replace("\r\n", "\n")
                        .chars()
                        .count();
                let preferred = if element.splat() {
                    Some(&Style::Splat)
                } else if element.rocket {
                    rocket.first()
                } else {
                    colon.first()
                };
                if let Some(delta) = preferred.and_then(|preferred| {
                    changes
                        .iter()
                        .find(|(s, i, _)| s == preferred && *i == index)
                        .map(|(_, _, d)| *d)
                }) {
                    if let (Some(target), Some(edits)) =
                        (corrections.as_deref_mut(), edits(source, element, delta))
                    {
                        diagnostic.corrected = !edits.is_empty();
                        target.extend(edits);
                    }
                }
                diagnostics.push(diagnostic);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::{
        assert_cop_no_offenses_full_with_config, assert_cop_offenses_full_with_config,
    };

    #[test]
    fn original_spec_states() {
        crate::testutil::assert_original_spec_parity(
            &HashAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/hash_alignment/original-parity.json"
            )),
        );
    }

    #[test]
    fn independent_original_states() {
        crate::testutil::assert_original_spec_parity(
            &HashAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/hash_alignment/independent-parity.json"
            )),
        );
    }

    #[test]
    fn original_source_loss_is_not_reproduced() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/hash_alignment/safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(
            &HashAlignment,
            &serde_json::to_string(&cases).unwrap(),
        );
    }

    crate::cop_fixture_tests!(HashAlignment, "cops/layout/hash_alignment");

    #[test]
    fn respects_fixed_argument_indentation() {
        let mut config = CopConfig::default();
        config.options.insert(
            "ArgumentAlignmentStyle".into(),
            "with_fixed_indentation".into(),
        );
        assert_cop_no_offenses_full_with_config(
            &HashAlignment,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/hash_alignment/fixed_argument_indent/no_offense.rb"
            )),
            config,
        );
    }

    fn style_config(style: &str) -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "EnforcedLastArgumentHashStyle".into(),
            serde_yml::Value::String(style.into()),
        );
        config
    }

    #[test]
    fn always_ignore_skips_last_arg_hash_and_kwargs() {
        let cfg = style_config("always_ignore");
        assert_cop_no_offenses_full_with_config(
            &HashAlignment,
            b"foo({\n  a: 1,\n    b: 2\n})\n",
            cfg.clone(),
        );
        assert_cop_no_offenses_full_with_config(
            &HashAlignment,
            b"foo(x,\n  a: 1,\n    b: 2)\n",
            cfg,
        );
    }

    #[test]
    fn always_ignore_still_checks_standalone_hash() {
        assert_cop_offenses_full_with_config(
            &HashAlignment,
            b"{\n  a: 1,\n    b: 2\n    ^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.\n}\n",
            style_config("always_ignore"),
        );
    }

    #[test]
    fn ignore_explicit_skips_braced_last_arg_keeps_kwargs() {
        assert_cop_no_offenses_full_with_config(
            &HashAlignment,
            b"foo({\n  a: 1,\n    b: 2\n})\n",
            style_config("ignore_explicit"),
        );
        assert_cop_offenses_full_with_config(
            &HashAlignment,
            b"foo(x,\n  a: 1,\n    b: 2\n    ^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.\n)\n",
            style_config("ignore_explicit"),
        );
    }

    #[test]
    fn ignore_implicit_skips_kwargs_keeps_braced_last_arg() {
        assert_cop_no_offenses_full_with_config(
            &HashAlignment,
            b"foo(x,\n  a: 1,\n    b: 2)\n",
            style_config("ignore_implicit"),
        );
        assert_cop_offenses_full_with_config(
            &HashAlignment,
            b"foo({\n  a: 1,\n    b: 2\n    ^^ Layout/HashAlignment: Align the keys of a hash literal if they span more than one line.\n})\n",
            style_config("ignore_implicit"),
        );
    }
}
