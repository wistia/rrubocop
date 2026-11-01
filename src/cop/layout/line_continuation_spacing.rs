//! Layout/LineContinuationSpacing.

use tree_sitter::Tree;

use crate::cop::layout::report;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct LineContinuationSpacing;

fn is_cont(bytes: &[u8], i: usize) -> bool {
    bytes[i] == b'\\' && matches!(bytes.get(i + 1), Some(b'\n') | Some(b'\r'))
}

fn has_space_before(bytes: &[u8], i: usize) -> bool {
    i > 0 && matches!(bytes[i - 1], b' ' | b'\t')
}

fn strip_start(bytes: &[u8], i: usize) -> usize {
    let mut s = i;
    while s > 0 && matches!(bytes[s - 1], b' ' | b'\t') {
        s -= 1;
    }
    s
}

fn check_at(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    i: usize,
    want: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let has_space = has_space_before(bytes, i);
    if want && !has_space {
        report::insert_space(
            cop,
            source,
            i,
            "Use one space before backslash for line continuation.".into(),
            diagnostics,
            corrections,
            i,
        );
    } else if !want && has_space {
        let s = strip_start(bytes, i);
        report::report_fix(
            cop,
            source,
            s,
            "Do not use space before backslash for line continuation.".into(),
            diagnostics,
            corrections,
            s,
            i,
            String::new(),
        );
    }
}

impl Cop for LineContinuationSpacing {
    fn name(&self) -> &'static str {
        "Layout/LineContinuationSpacing"
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
        tree: &Tree,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = tree;
        let want = config.get_str("EnforcedStyle", "space") != "no_space";
        let bytes = source.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if is_cont(bytes, i) && !code_map.covers(i) {
                check_at(self, source, bytes, i, want, diagnostics, &mut corrections);
            }
            i += 1;
        }
    }
}
