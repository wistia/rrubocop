use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Lint/OrderedMagicComments — encoding before other magic comments.
pub struct OrderedMagicComments;

fn is_encoding(line: &str) -> bool {
    encoding_magic(&line.to_ascii_lowercase())
}

fn encoding_magic(line: &str) -> bool {
    let content = line
        .trim_start_matches('#')
        .trim()
        .trim_start_matches("-*-")
        .trim();
    content.starts_with("encoding:")
        || content.starts_with("coding:")
        || content.starts_with("encoding =")
}

fn is_other_magic(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("frozen_string_literal") || lower.contains("shareable_constant_value")
}

fn first_disorder(source: &SourceFile) -> Option<usize> {
    // The first "other magic" comment seen before the encoding comment is
    // out of order; nothing after the encoding comment can matter.
    let mut other_line = None;
    for (i, line) in source.lines().enumerate() {
        match String::from_utf8_lossy(line).trim() {
            t if t.is_empty() => {}
            t if !t.starts_with('#') => break,
            t => {
                if is_encoding(t) {
                    return other_line;
                } else if is_other_magic(t) {
                    other_line = Some(i + 1);
                }
            }
        }
    }
    None
}

impl Cop for OrderedMagicComments {
    fn name(&self) -> &'static str {
        "Lint/OrderedMagicComments"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _tree: &tree_sitter::Tree,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(line) = first_disorder(source) else {
            return;
        };
        diagnostics.push(self.diagnostic(
            source,
            line,
            0,
            "The encoding magic comment should precede all other magic comments.".to_string(),
        ));
    }
}
