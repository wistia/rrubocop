//! Style/FrozenStringLiteralComment.

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FrozenStringLiteralComment;

fn is_frozen_magic(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('#')
        && (t.contains("frozen_string_literal:") || t.contains("frozen-string-literal:"))
        && (t.contains("true") || t.contains("false"))
}

fn is_leading_trivia(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

fn has_frozen_comment(source: &SourceFile) -> bool {
    for line in source.lines() {
        let Ok(s) = std::str::from_utf8(line) else {
            continue;
        };
        if is_frozen_magic(s) {
            return true;
        }
        if !is_leading_trivia(s) {
            break;
        }
    }
    false
}

fn insert_offset(bytes: &[u8]) -> usize {
    if !bytes.starts_with(b"#!") {
        return 0;
    }
    bytes
        .iter()
        .position(|&b| b == b'\n')
        .map_or(0, |nl| nl + 1)
}

impl Cop for FrozenStringLiteralComment {
    fn name(&self) -> &'static str {
        "Style/FrozenStringLiteralComment"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_str("EnforcedStyle", "always") == "never" || source.as_bytes().is_empty() {
            return;
        }
        if has_frozen_comment(source) {
            return;
        }
        let mut diag = self.diagnostic(
            source,
            1,
            0,
            "Missing magic comment # frozen_string_literal: true.".to_string(),
        );
        if let Some(corr) = corrections {
            let start = insert_offset(source.as_bytes());
            corr.push(Correction {
                start,
                end: start,
                replacement: "# frozen_string_literal: true\n".into(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
