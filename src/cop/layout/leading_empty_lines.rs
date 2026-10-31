//! Layout/LeadingEmptyLines.

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LeadingEmptyLines;

impl Cop for LeadingEmptyLines {
    fn name(&self) -> &'static str {
        "Layout/LeadingEmptyLines"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn uses_line_phase(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();
        if bytes.is_empty() || bytes[0] != b'\n' {
            return;
        }
        let mut end = 0usize;
        while end < bytes.len() && bytes[end] == b'\n' {
            end += 1;
        }
        if end == 0 {
            return;
        }
        let mut diag = self.diagnostic(
            source,
            1,
            0,
            "Unnecessary leading blank lines detected.".to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            corr.push(crate::correction::Correction {
                start: 0,
                end,
                replacement: String::new(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
