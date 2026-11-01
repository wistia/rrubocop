//! Layout/SpaceBeforeComma — no space before `,` in code.

use tree_sitter::Tree;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeComma;

fn space_run_before(bytes: &[u8], comma: usize) -> Option<usize> {
    if comma == 0 || !matches!(bytes[comma - 1], b' ' | b'\t') {
        return None;
    }
    let mut start = comma - 1;
    while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
        start -= 1;
    }
    Some(start)
}

fn report(
    cop: &SpaceBeforeComma,
    source: &SourceFile,
    start: usize,
    comma: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(start);
    let mut diag = cop.diagnostic(source, line, col, "Space found before comma.".to_string());
    if let Some(corr) = corrections.as_deref_mut() {
        corr.push(Correction {
            start,
            end: comma,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

impl Cop for SpaceBeforeComma {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeComma"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let bytes = source.as_bytes();
        for i in 1..bytes.len() {
            if bytes[i] != b',' || code_map.covers(i) {
                continue;
            }
            if let Some(start) = space_run_before(bytes, i) {
                report(self, source, start, i, diagnostics, &mut corrections);
            }
        }
    }
}
