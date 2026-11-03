use crate::cop::rspec::description;
use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::sync::LazyLock;
use tree_sitter::Node;

static SHOULD: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)^should(?:n't|n’t)?\b").unwrap());
static WILL: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)^(?:will|won't|won’t)\b").unwrap());
static IT: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)^it ").unwrap());
static NEGATIVE: LazyLock<Vec<(regex::Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (
            regex::Regex::new(r"(?i)^should(?:n't|n’t| not)\b be\b").unwrap(),
            "is not",
        ),
        (
            regex::Regex::new(r"(?i)^should(?:n't|n’t| not)\b").unwrap(),
            "does not",
        ),
        (regex::Regex::new(r"(?i)^will not\b").unwrap(), "does not"),
        (
            regex::Regex::new(r"(?i)^wo(?:n't|n’t)\b").unwrap(),
            "does not",
        ),
    ]
});
static ES: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)(?:o|s|x|ch|sh|z)$").unwrap());
fn uppercase(text: &str) -> bool {
    text.to_uppercase() == text
}
fn rewrite(text: &str, config: &CopConfig) -> String {
    for (pattern, replacement) in NEGATIVE.iter() {
        if let Some(prefix) = pattern.find(text) {
            let replacement = if uppercase(prefix.as_str()) {
                replacement.to_uppercase()
            } else {
                replacement.to_string()
            };
            return format!("{replacement}{}", &text[prefix.end()..]);
        }
    }
    let ignored = string_options(config, "IgnoredWords");
    let mut words: Vec<_> = text
        .split_ascii_whitespace()
        .skip(1)
        .map(str::to_string)
        .collect();
    for word in &mut words {
        if ignored
            .iter()
            .any(|ignored| ignored.to_lowercase() == word.to_lowercase())
        {
            continue;
        }
        if let Some(replacement) = config
            .options
            .get("CustomTransform")
            .and_then(serde_yml::Value::as_mapping)
            .and_then(|map| map.get(serde_yml::Value::String(word.clone())))
            .and_then(serde_yml::Value::as_str)
        {
            *word = replacement.into();
        } else {
            let mut chars = word.chars().rev();
            let last = chars.next();
            let before = chars.next();
            let ies = last.is_some_and(|character| character.eq_ignore_ascii_case(&'y'))
                && before.is_some_and(|character| {
                    !matches!(character.to_ascii_lowercase(), 'a' | 'e' | 'o' | 'u')
                });
            let suffix = if ES.is_match(word) {
                "es"
            } else if ies {
                "ies"
            } else {
                "s"
            };
            let suffix = if uppercase(word) {
                suffix.to_uppercase()
            } else {
                suffix.into()
            };
            if ies {
                word.pop();
            }
            word.push_str(&suffix);
        }
        break;
    }
    words.join(" ")
}
fn preprocess(text: &str) -> String {
    let mut result = String::new();
    for character in text
        .trim_matches(|character| {
            matches!(character, '\0' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' ')
        })
        .chars()
    {
        if character != ' ' || !result.ends_with(' ') {
            result.push(character);
        }
    }
    result.to_lowercase()
}
enum WordingReplacement {
    Transform,
    StripPrefix(usize),
}
pub struct ExampleWording;
impl Cop for ExampleWording {
    fn name(&self) -> &'static str {
        "RSpec/ExampleWording"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"it"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if name(source, node) != Some(b"it")
            || safe_send(source, node)
            || call_block(node).is_none()
            || numbered_block(source, node)
        {
            return;
        }
        let Some(description) = call_args(node).first().copied() else {
            return;
        };
        if !description::first_static(source, description, false) {
            return;
        }
        let Some(text) = description::text(source, description) else {
            return;
        };
        let (message, replacement) = if SHOULD.is_match(&text) {
            (
                "Do not use should when describing your tests.",
                Some(WordingReplacement::Transform),
            )
        } else if WILL.is_match(&text) {
            (
                "Do not use the future tense when describing your tests.",
                Some(WordingReplacement::Transform),
            )
        } else if let Some(prefix) = IT.find(&text) {
            (
                "Do not repeat 'it' when describing your tests.",
                Some(WordingReplacement::StripPrefix(prefix.end())),
            )
        } else if string_options(config, "DisallowedExamples")
            .iter()
            .any(|example| preprocess(example) == preprocess(&text))
        {
            ("Your example description is insufficient.", None)
        } else {
            return;
        };
        let start = description.start_byte() + 1;
        let end = description.end_byte().saturating_sub(1).max(start);
        let index = report(self, source, start, end, message.into(), diagnostics);
        diagnostics[index].correctable =
            replacement.is_some() && description.kind() != "heredoc_beginning";
        if corrections.is_none() {
            return;
        }
        let replacement = replacement.map(|replacement| match replacement {
            WordingReplacement::Transform => rewrite(&text, config),
            WordingReplacement::StripPrefix(start) => text[start..].to_owned(),
        });
        if let Some(replacement) = replacement
            .filter(|replacement| description::safe_replacement(source, description, replacement))
        {
            if preserving_replace(
                source,
                description,
                &mut corrections,
                start,
                end,
                replacement,
                self.name(),
            ) {
                diagnostics[index].corrected = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rewriting_matches_original_wording_unit_specimens() {
        let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/rspec-parity/wording-rewrites.json"
        ))
        .unwrap();
        let mut config = CopConfig::default();
        config.options.insert(
            "IgnoredWords".into(),
            serde_yml::from_str("[only, really]").unwrap(),
        );
        config.options.insert(
            "CustomTransform".into(),
            serde_yml::from_str("{have: has}").unwrap(),
        );
        for case in cases {
            assert_eq!(
                rewrite(case["old"].as_str().unwrap(), &config),
                case["new"].as_str().unwrap(),
                "{}",
                case["old"]
            );
        }
    }
}
