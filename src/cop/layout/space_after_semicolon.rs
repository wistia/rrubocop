//! Layout/SpaceAfterSemicolon — ported from RuboCop/nitrocop (tree-sitter).

use tree_sitter::Tree;

use crate::cop::layout::report;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceAfterSemicolon;

fn needs_space_after(bytes: &[u8], i: usize) -> bool {
    if matches!(bytes.get(i.wrapping_sub(1)), Some(b'$')) {
        return false;
    }
    let next = bytes.get(i + 1).copied();
    if matches!(
        next,
        Some(b';') | Some(b'\\') | Some(b')') | Some(b']') | Some(b'|')
    ) {
        return false;
    }
    !matches!(
        next,
        Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') | None
    )
}

impl Cop for SpaceAfterSemicolon {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterSemicolon"
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
            if b != b';' || code_map.covers(i) || !needs_space_after(bytes, i) {
                continue;
            }
            report::insert_space(
                self,
                source,
                i,
                "Space missing after semicolon.".into(),
                diagnostics,
                &mut corrections,
                i + 1,
            );
        }
    }
}
