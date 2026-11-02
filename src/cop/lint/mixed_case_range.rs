//! Lint/MixedCaseRange.

use tree_sitter::Node;

use crate::cop::shared::{node_text, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MixedCaseRange;

const MESSAGE: &str = "Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.";

fn single_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(character), None) => Some(character),
        _ => None,
    }
}

fn range_chars(source: &SourceFile, node: Node<'_>) -> Option<(char, char)> {
    let begin = node.child_by_field_name("begin")?;
    let end = node.child_by_field_name("end")?;
    let character = |node| single_char(&crate::cop::ported::static_ruby_string(source, node)?);
    Some((character(begin)?, character(end)?))
}

fn character_class(character: char) -> Option<bool> {
    character
        .is_ascii_alphabetic()
        .then_some(character.is_ascii_uppercase())
}

fn replacement(first: u8, last: u8) -> String {
    let first_end = if first.is_ascii_uppercase() {
        b'Z'
    } else {
        b'z'
    };
    let last_start = if last.is_ascii_uppercase() {
        b'A'
    } else {
        b'a'
    };
    let mut result = String::new();
    result.push(char::from(first));
    if first != first_end {
        result.push('-');
        result.push(char::from(first_end));
    }
    result.push(char::from(last_start));
    if last_start != last {
        result.push('-');
        result.push(char::from(last));
    }
    result
}

impl Cop for MixedCaseRange {
    fn name(&self) -> &'static str {
        "Lint/MixedCaseRange"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["range", "regex"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if node.kind() == "range" {
            let Some((first, last)) = range_chars(source, node) else {
                return;
            };
            if character_class(first) == character_class(last) {
                return;
            }
            let (line, column) = source.offset_to_line_col(node.start_byte());
            let mut diagnostic = self.diagnostic(source, line, column, MESSAGE.into());
            diagnostic.highlight_length = node_text(source, node).chars().count();
            diagnostic.correctable = false;
            diagnostics.push(diagnostic);
        } else {
            for (start, end) in crate::cop::ported::mixed_case_regexp_ranges(source, node) {
                let bytes = &source.as_bytes()[start..end];
                let (line, column) = source.offset_to_line_col(start);
                let mut diagnostic = self.diagnostic(source, line, column, MESSAGE.into());
                diagnostic.highlight_length = String::from_utf8_lossy(bytes).chars().count();
                diagnostic.corrected = push_replace(
                    &mut corrections,
                    start,
                    end,
                    replacement(bytes[0], *bytes.last().unwrap()),
                    self.name(),
                );
                diagnostics.push(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &MixedCaseRange,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/mixed_case_range/parity.json"
            )),
        );
    }
}
