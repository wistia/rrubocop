//! Layout/SpaceAfterComma — require whitespace after `,` in code (not strings/comments).

use tree_sitter::Tree;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceAfterComma;

fn ok_after_comma(next: Option<u8>) -> bool {
    matches!(
        next,
        Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') | None
    )
}

fn skip_before_close(next: Option<u8>, skip_rcurly: bool) -> bool {
    matches!(next, Some(b')') | Some(b']') | Some(b'|') | Some(b';'))
        || (skip_rcurly && next == Some(b'}'))
}

fn report_missing(
    cop: &SpaceAfterComma,
    source: &SourceFile,
    at: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(at);
    let mut diag = cop.diagnostic(source, line, col, "Space missing after comma.".to_string());
    if let Some(corr) = corrections.as_deref_mut() {
        corr.push(Correction {
            start: at + 1,
            end: at + 1,
            replacement: " ".into(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn line_continued(bytes: &[u8], i: usize) -> bool {
    bytes.get(i + 1) == Some(&b'\\')
        && matches!(bytes.get(i + 2).copied(), Some(b'\n') | Some(b'\r') | None)
}

fn should_flag(bytes: &[u8], i: usize, code_map: &CodeMap, skip_rcurly: bool) -> bool {
    if bytes[i] != b',' || code_map.covers(i) || (i > 0 && bytes[i - 1] == b'$') {
        return false;
    }
    let next = bytes.get(i + 1).copied();
    if skip_before_close(next, skip_rcurly) || line_continued(bytes, i) {
        return false;
    }
    !ok_after_comma(next)
}

impl Cop for SpaceAfterComma {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterComma"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn needs_code_map(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _tree: &Tree,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let skip_rcurly = config.get_str("__SpaceInsideHashBracesStyle", "space") == "no_space";
        let bytes = source.as_bytes();
        for i in 0..bytes.len() {
            if should_flag(bytes, i, code_map, skip_rcurly) {
                report_missing(self, source, i, diagnostics, &mut corrections);
            }
        }
    }
}

#[cfg(test)]
mod tests;
