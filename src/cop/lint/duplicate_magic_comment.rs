//! Lint/DuplicateMagicComment — remove duplicate encoding/FSL comments.

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct DuplicateMagicComment;

fn magic_kind(line: &str) -> Option<&'static str> {
    let t = line.trim();
    if !t.starts_with('#') {
        return None;
    }
    // After `#`, require a real magic key — skip `# # frozen_string_literal: …`
    // (commented-out magic comments are not active).
    let after_hash = t[1..].trim_start();
    if after_hash.starts_with('#') {
        return None;
    }
    let lower = after_hash.to_ascii_lowercase();
    if lower.starts_with("frozen_string_literal:") || lower.starts_with("frozen-string-literal:") {
        Some("frozen_string_literal")
    } else if is_encoding_magic(&lower) {
        Some("encoding")
    } else if lower.starts_with("shareable_constant_value:") {
        Some("shareable_constant_value")
    } else {
        None
    }
}

/// True encoding magic comments only — not prose mentioning `Encoding::…`.
fn is_encoding_magic(lower_after_hash: &str) -> bool {
    let s = lower_after_hash.trim_start_matches(|c: char| c == '-' || c == '*' || c == ' ');
    s.starts_with("encoding:")
        || s.starts_with("coding:")
        || s.starts_with("encoding =")
        || (s.contains("-*-") && (s.contains("encoding:") || s.contains("coding:")))
}

fn line_span(offset: usize, line: &[u8], total: usize) -> (usize, usize) {
    let end = offset + line.len() + usize::from(offset + line.len() < total);
    (offset, end)
}

fn still_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

fn report_dup(
    cop: &DuplicateMagicComment,
    source: &SourceFile,
    line_no: usize,
    start: usize,
    end: usize,
    corrections: &mut Option<&mut Vec<Correction>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut diag = cop.diagnostic(
        source,
        line_no,
        0,
        "Duplicate magic comment detected.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start,
            end,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn process_line(
    cop: &DuplicateMagicComment,
    source: &SourceFile,
    line: &[u8],
    line_no: usize,
    offset: usize,
    seen: &mut std::collections::HashSet<&'static str>,
    corrections: &mut Option<&mut Vec<Correction>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<usize> {
    let (line_start, line_end) = line_span(offset, line, source.as_bytes().len());
    let s = String::from_utf8_lossy(line);
    if !still_header(&s) {
        return None;
    }
    if let Some(kind) = magic_kind(&s)
        && !seen.insert(kind)
    {
        report_dup(
            cop,
            source,
            line_no,
            line_start,
            line_end,
            corrections,
            diagnostics,
        );
    }
    Some(line_end)
}

impl Cop for DuplicateMagicComment {
    fn name(&self) -> &'static str {
        "Lint/DuplicateMagicComment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        _tree: &tree_sitter::Tree,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let mut seen = std::collections::HashSet::new();
        let mut offset = 0usize;
        for (i, line) in source.lines().enumerate() {
            match process_line(
                self,
                source,
                line,
                i + 1,
                offset,
                &mut seen,
                &mut corrections,
                diagnostics,
            ) {
                Some(next) => offset = next,
                None => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateMagicComment, "cops/lint/duplicate_magic_comment");
}
