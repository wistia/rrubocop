//! Layout/SpaceBeforeSemicolon — ported from RuboCop/nitrocop (tree-sitter).

use tree_sitter::Tree;

use crate::cop::layout::report;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeSemicolon;

fn ws_before_semi(bytes: &[u8], i: usize) -> Option<usize> {
    let line_start = bytes[..i]
        .iter()
        .rposition(|&c| c == b'\n')
        .map_or(0, |x| x + 1);
    let mut ws = i;
    while ws > line_start && matches!(bytes[ws - 1], b' ' | b'\t') {
        ws -= 1;
    }
    if ws == i || ws == line_start || bytes[ws - 1] == b'{' {
        None
    } else {
        Some(ws)
    }
}

fn check_at(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    i: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some(ws) = ws_before_semi(bytes, i) else {
        return;
    };
    report::report_fix(
        cop,
        source,
        ws,
        "Space found before semicolon.".into(),
        diagnostics,
        corrections,
        ws,
        i,
        String::new(),
    );
}

impl Cop for SpaceBeforeSemicolon {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeSemicolon"
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
        for (i, &b) in bytes.iter().enumerate() {
            if b != b';' || i == 0 || code_map.covers(i) {
                continue;
            }
            check_at(self, source, bytes, i, diagnostics, &mut corrections);
        }
    }
}
