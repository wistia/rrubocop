//! RSpec/IndexedLet — flags `let`/`let!` names carrying a numeric index
//! (`item1`, `item_2`, `trigger_at_72`, ...).
//!
//! Port of rubocop-rspec 3.6 semantics: lets whose names differ only by
//! digits are grouped, and a group is flagged when its size exceeds `Max`
//! (default 1). A single indexed name alone is therefore not an offense.

use std::collections::HashMap;

use regex::Regex;
use tree_sitter::Node;

use crate::cop::rspec::helpers::{
    RSPEC_INCLUDE, bare_rspec_call, block_body, call_block, first_sym_arg, is_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct IndexedLet;

/// /_?\d+$/ — trailing digits with an optional underscore before them.
fn ends_with_index(name: &[u8]) -> bool {
    name.last().is_some_and(|b| b.is_ascii_digit())
}

/// First digit run of the name — what the upstream message quotes.
fn first_digit_run(name: &[u8]) -> Option<&[u8]> {
    let rest = &name[name.iter().position(|b| b.is_ascii_digit())?..];
    match rest.iter().position(|b| !b.is_ascii_digit()) {
        Some(i) => Some(&rest[..i]),
        None => Some(rest),
    }
}

/// Upstream grouping key: the name with every digit run stripped.
fn strip_digits(name: &[u8]) -> Vec<u8> {
    name.iter()
        .copied()
        .filter(|b| !b.is_ascii_digit())
        .collect()
}

fn is_let_call(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(node.kind(), "call" | "command")
        && bare_rspec_call(source, node).is_some_and(|m| matches!(m, b"let" | b"let!"))
}

fn option_strings(config: &CopConfig, key: &str) -> Vec<String> {
    config
        .options
        .get(key)
        .and_then(serde_yml::Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn is_allowed(name: &[u8], identifiers: &[String], patterns: &[Regex]) -> bool {
    identifiers.iter().any(|a| a.as_bytes() == name)
        || patterns
            .iter()
            .any(|re| re.is_match(&String::from_utf8_lossy(name)))
}

/// Direct `let`/`let!` statements of one spec group whose names end with
/// an index.
fn indexed_let_names<'a, 'b>(source: &'a SourceFile, body: Node<'b>) -> Vec<(&'a [u8], Node<'b>)> {
    let mut cur = body.walk();
    body.named_children(&mut cur)
        .filter(|stmt| is_let_call(source, *stmt))
        .filter_map(|stmt| first_sym_arg(source, stmt).map(|name| (name, stmt)))
        .filter(|(name, _)| ends_with_index(name))
        .collect()
}

/// `indexed_let_names` minus names exempted by `AllowedIdentifiers`
/// and `AllowedPatterns`.
fn collect_indexed_lets<'a, 'b>(
    source: &'a SourceFile,
    body: Node<'b>,
    config: &CopConfig,
) -> Vec<(&'a [u8], Node<'b>)> {
    let identifiers = option_strings(config, "AllowedIdentifiers");
    let patterns = super::parity_helpers::compiled_patterns(source, config, "AllowedPatterns");
    indexed_let_names(source, body)
        .into_iter()
        .filter(|(name, _)| !is_allowed(name, &identifiers, &patterns))
        .collect()
}

/// Members grouped by digit-stripped name, ordered by first appearance.
fn group_by_stripped_name<'a>(indexed: &[(&'a [u8], Node<'a>)]) -> Vec<Vec<(&'a [u8], Node<'a>)>> {
    let mut groups: HashMap<Vec<u8>, Vec<(&'a [u8], Node<'a>)>> = HashMap::new();
    for (name, stmt) in indexed {
        groups
            .entry(strip_digits(name))
            .or_default()
            .push((*name, *stmt));
    }
    let mut grouped: Vec<_> = groups.into_values().collect();
    grouped.sort_by_key(|members| {
        members
            .first()
            .map_or(usize::MAX, |(_, node)| node.start_byte())
    });
    grouped
}

fn report_group(
    cop: &IndexedLet,
    source: &SourceFile,
    members: &[(&[u8], Node<'_>)],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (name, stmt) in members {
        let Some(index) = first_digit_run(name) else {
            continue;
        };
        let index = String::from_utf8_lossy(index);
        let (line, col) = source.offset_to_line_col(stmt.start_byte());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            col,
            format!(
                "This `let` statement uses `{index}` in its name. Please give it a meaningful name."
            ),
        ));
    }
}

fn report_oversized_groups(
    cop: &IndexedLet,
    source: &SourceFile,
    indexed: &[(&[u8], Node<'_>)],
    max: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for members in group_by_stripped_name(indexed) {
        if members.len() > max {
            report_group(cop, source, &members, diagnostics);
        }
    }
}

impl Cop for IndexedLet {
    fn name(&self) -> &'static str {
        "RSpec/IndexedLet"
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(method) = bare_rspec_call(source, node) else {
            return;
        };
        if !is_group(method) {
            return;
        }
        let Some(body) = call_block(node).and_then(block_body) else {
            return;
        };
        report_oversized_groups(
            self,
            source,
            &collect_indexed_lets(source, body, config),
            config.get_usize("Max", 1),
            diagnostics,
        );
    }

    fn redundant_disable_audit(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IndexedLet, "cops/rspec/indexed_let");

    #[test]
    fn single_indexed_let_is_allowed_by_default_max() {
        let diags = crate::testutil::run_cop_full(
            &IndexedLet,
            b"RSpec.describe Foo do\n  let(:item2) { build(:item) }\nend\n",
        );
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn max_zero_flags_even_a_single_indexed_let() {
        let config = CopConfig {
            options: HashMap::from([("Max".to_string(), serde_yml::Value::from(0))]),
            ..Default::default()
        };
        let diags = crate::testutil::run_cop_full_with_config(
            &IndexedLet,
            b"RSpec.describe Foo do\n  let(:item2) { build(:item) }\nend\n",
            config,
        );
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].location.column, 2);
    }

    #[test]
    fn allowed_identifiers_suppress_the_offense() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowedIdentifiers".to_string(),
                serde_yml::Value::from(vec!["item2", "item3"]),
            )]),
            ..Default::default()
        };
        let diags = crate::testutil::run_cop_full_with_config(
            &IndexedLet,
            b"RSpec.describe Foo do\n  let(:item2) { build(:item) }\n  let(:item3) { build(:item) }\nend\n",
            config,
        );
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn allowed_patterns_suppress_the_offense() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".to_string(),
                serde_yml::Value::from(vec!["^item"]),
            )]),
            ..Default::default()
        };
        let diags = crate::testutil::run_cop_full_with_config(
            &IndexedLet,
            b"RSpec.describe Foo do\n  let(:item2) { build(:item) }\n  let(:item3) { build(:item) }\nend\n",
            config,
        );
        assert!(diags.is_empty(), "{diags:?}");
    }
}
