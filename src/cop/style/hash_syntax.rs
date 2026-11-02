//! Style/HashSyntax.

use crate::cop::shared::{node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

mod shorthand;

pub struct HashSyntax;

fn pairs(node: Node<'_>) -> Vec<Node<'_>> {
    let Some(parent) = node.parent() else {
        return vec![node];
    };
    let mut cursor = parent.walk();
    parent
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "pair")
        .collect()
}

fn operator(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| matches!(child.kind(), ":" | "=>"))
}

fn acceptable(source: &SourceFile, pair: Node<'_>, config: &CopConfig) -> bool {
    let Some(key) = pair.child_by_field_name("key") else {
        return false;
    };

    let recovered_symbol = key.kind() == "identifier"
        && key.start_byte() > 0
        && source.as_bytes()[key.start_byte() - 1] == b':';
    if !recovered_symbol
        && !matches!(
            key.kind(),
            "simple_symbol" | "delimited_symbol" | "hash_key_symbol"
        )
        && !(key.kind() == "string" && operator(pair).is_some_and(|op| op.kind() == ":"))
    {
        return false;
    }
    let Ok(text) = std::str::from_utf8(node_bytes(source, key)) else {
        return false;
    };
    let text = text.strip_prefix(':').unwrap_or(text);
    if config.get_bool("PreferHashRocketsForNonAlnumEndingSymbols", false)
        && !text
            .chars()
            .last()
            .is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '\'' | '"'))
    {
        return false;
    }
    if super::heuristics::word_symbol_name(text.as_bytes()) {
        return true;
    }
    config.get_f64("TargetRubyVersion", 2.7) > 2.1
        && ((text.starts_with('"') && text.ends_with('"'))
            || (text.starts_with('\'') && text.ends_with('\'')))
}

impl Cop for HashSyntax {
    fn name(&self) -> &'static str {
        "Style/HashSyntax"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["pair"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        shorthand::check(
            source,
            node,
            config,
            diagnostics,
            corrections.as_deref_mut(),
        );
        let Some(op) = operator(node) else {
            return;
        };
        let siblings = pairs(node);
        let all_symbols = siblings
            .iter()
            .all(|pair| acceptable(source, *pair, config));
        let force_rockets = config.get_bool("UseHashRocketsWithSymbolValues", false)
            && siblings.iter().any(|pair| {
                pair.child_by_field_name("value").is_some_and(|value| {
                    if !matches!(value.kind(), "simple_symbol" | "delimited_symbol") {
                        return false;
                    }
                    let mut interpolation = false;
                    crate::cop::shared::for_each_descendant(value, |child| {
                        interpolation |= child.kind() == "interpolation"
                    });
                    !interpolation
                })
            });
        let style = config.get_str("EnforcedStyle", "ruby19");
        let (wanted, message) = if style == "hash_rockets" || force_rockets {
            ("=>", "Use hash rockets syntax.")
        } else if style == "no_mixed_keys" {
            (
                if all_symbols {
                    siblings
                        .first()
                        .and_then(|pair| operator(*pair))
                        .map_or("=>", |op| op.kind())
                } else {
                    "=>"
                },
                "Don't mix styles in the same hash.",
            )
        } else if all_symbols {
            (":", "Use the new Ruby 1.9 hash syntax.")
        } else if style == "ruby19_no_mixed_keys" {
            ("=>", "Don't mix styles in the same hash.")
        } else {
            return;
        };
        if op.kind() == wanted {
            return;
        }
        let Some(key) = node.child_by_field_name("key") else {
            return;
        };
        let start = if key.kind() == "identifier"
            && key.start_byte() > 0
            && source.as_bytes()[key.start_byte() - 1] == b':'
        {
            key.start_byte() - 1
        } else {
            node.start_byte()
        };
        let (line, column) = source.offset_to_line_col(start);
        let mut diagnostic = self.diagnostic(source, line, column, message.into());
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[start..op.end_byte()])
                .chars()
                .count();
        let bytes = source.as_bytes();
        if wanted == ":" {
            let mut end = op.end_byte();
            while bytes
                .get(end)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                end += 1;
            }
            while bytes.get(end) == Some(&b'\n') {
                end += 1;
            }
            let text = node_bytes(source, key)
                .strip_prefix(b":")
                .unwrap_or(node_bytes(source, key));
            let prefix = node
                .parent()
                .filter(|parent| parent.kind() == "argument_list")
                .and_then(|parent| parent.parent())
                .and_then(|call| call.child_by_field_name("method"))
                .is_some_and(|method| method.end_byte() == key.start_byte());
            diagnostic.corrected = push_replace(
                &mut corrections,
                start,
                end,
                format!(
                    "{}{}: ",
                    if prefix { " " } else { "" },
                    String::from_utf8_lossy(text)
                ),
                self.name(),
            );
            if let Some(parent) = node
                .parent()
                .filter(|parent| parent.kind() == "argument_list")
            {
                if parent
                    .parent()
                    .is_some_and(|parent| parent.kind() == "return")
                    && siblings
                        .iter()
                        .find(|pair| operator(**pair).is_some_and(|op| op.kind() == "=>"))
                        .is_some_and(|first| first.id() == node.id())
                {
                    push_replace(
                        &mut corrections,
                        siblings[0].start_byte(),
                        siblings[0].start_byte(),
                        "{",
                        self.name(),
                    );
                    let end = siblings.last().unwrap().end_byte();
                    push_replace(&mut corrections, end, end, "}", self.name());
                }
            }
        } else {
            let text = String::from_utf8_lossy(node_bytes(source, key));
            let mut end = op.end_byte();
            while bytes
                .get(end)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                end += 1;
            }
            while bytes.get(end) == Some(&b'\n') {
                end += 1;
            }
            let spacing = " => ";
            let omitted = node.child_by_field_name("value").is_none();
            diagnostic.corrected = push_replace(
                &mut corrections,
                start,
                end,
                format!(
                    ":{text}{spacing}{}",
                    if omitted { text.as_ref() } else { "" }
                ),
                self.name(),
            );
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn additional_original_argument_scopes() {
        crate::testutil::assert_original_spec_parity(
            &HashSyntax,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/hash_syntax/shorthand-array.json"
            )),
        );
    }
    #[test]
    fn shorthand_source_safeguards() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/hash_syntax/shorthand-safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&HashSyntax, &cases.to_string());
    }
    #[test]
    fn independent_original_shorthand_states() {
        crate::testutil::assert_original_spec_parity(
            &HashSyntax,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/hash_syntax/shorthand-extra.json"
            )),
        );
    }
    #[test]
    fn original_shorthand_specs() {
        crate::testutil::assert_original_spec_parity(
            &HashSyntax,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/hash_syntax/shorthand.json"
            )),
        );
    }
    #[test]
    fn returned_hash_gets_one_brace_pair_instead_of_invalid_nested_hash() {
        let mut fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/hash_syntax/intentional-safeguards.json"
        )))
        .unwrap();
        for case in fixture.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&HashSyntax, &fixture.to_string());
    }

    #[test]
    fn original_ruby_hash_delimiter_specs() {
        crate::testutil::assert_original_spec_parity(
            &HashSyntax,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/hash_syntax/parity.json"
            )),
        );
    }
}
