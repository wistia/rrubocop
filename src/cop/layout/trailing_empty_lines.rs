//! Layout/TrailingEmptyLines — adapted from nitrocop.

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingEmptyLines;

fn contains_end_marker(bytes: &[u8]) -> bool {
    bytes.windows(7).any(|w| w == b"__END__")
}

fn trailing_whitespace_info(bytes: &[u8]) -> (usize, usize) {
    let mut ws_len = 0;
    let mut newline_count = 0;
    for &b in bytes.iter().rev() {
        if matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\x0b' | b'\x0c') {
            ws_len += 1;
            if b == b'\n' {
                newline_count += 1;
            }
        } else {
            break;
        }
    }
    (ws_len, newline_count)
}

fn first_trailing_blank_line_offset(bytes: &[u8], trailing_start: usize) -> usize {
    let mut offset = trailing_start;
    if bytes.get(offset) == Some(&b'\r') {
        offset += 1;
    }
    if bytes.get(offset) == Some(&b'\n') {
        offset += 1;
    }
    offset.min(bytes.len().saturating_sub(1))
}

fn missing_blank_loc(source: &SourceFile, bytes: &[u8], trailing_start: usize) -> (usize, usize) {
    if matches!(bytes.get(trailing_start), Some(b'\n' | b'\r')) {
        let (last_line, _) = source.offset_to_line_col(trailing_start);
        (last_line + 1, 0)
    } else {
        source.offset_to_line_col(trailing_start + 1)
    }
}

fn blank_message(blank_lines: isize, wanted: isize) -> String {
    match blank_lines {
        -1 => "Final newline missing.".into(),
        0 => "Trailing blank line missing.".into(),
        1 => "Trailing blank line detected.".into(),
        n if wanted == 0 => format!("{n} trailing blank lines detected."),
        n => format!("{n} trailing blank lines instead of {wanted} detected."),
    }
}

fn report_loc(
    source: &SourceFile,
    bytes: &[u8],
    begin_pos: usize,
    blank_lines: isize,
    ws_len: usize,
) -> (usize, usize) {
    if blank_lines == 0 {
        missing_blank_loc(source, bytes, begin_pos)
    } else if blank_lines > 0 {
        source.offset_to_line_col(first_trailing_blank_line_offset(bytes, begin_pos))
    } else if ws_len > 0 {
        source.offset_to_line_col(begin_pos + 1)
    } else {
        source.offset_to_line_col(begin_pos)
    }
}

fn apply_fix(cop: &dyn Cop, style: &str, begin_pos: usize, end: usize, corr: &mut Vec<Correction>) {
    let replacement = if style == "final_blank_line" {
        "\n\n".into()
    } else {
        "\n".into()
    };
    corr.push(Correction {
        start: begin_pos,
        end,
        replacement,
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
}

fn trailing_offense(
    source: &SourceFile,
    bytes: &[u8],
    style: &str,
) -> Option<(usize, usize, String, usize)> {
    if bytes.is_empty() || contains_end_marker(bytes) || bytes.ends_with(b"%\n\n") {
        return None;
    }
    let (ws_len, newline_count) = trailing_whitespace_info(bytes);
    let blank_lines = newline_count as isize - 1;
    let wanted: isize = if style == "final_blank_line" { 1 } else { 0 };
    if blank_lines == wanted {
        return None;
    }
    let begin_pos = bytes.len() - ws_len;
    let (rl, rc) = report_loc(source, bytes, begin_pos, blank_lines, ws_len);
    Some((rl, rc, blank_message(blank_lines, wanted), begin_pos))
}

impl Cop for TrailingEmptyLines {
    fn name(&self) -> &'static str {
        "Layout/TrailingEmptyLines"
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
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "final_newline");
        let bytes = source.as_bytes();
        let Some((rl, rc, msg, begin_pos)) = trailing_offense(source, bytes, style) else {
            return;
        };
        let mut diag = self.diagnostic(source, rl, rc, msg);
        if let Some(corr) = corrections.as_mut() {
            apply_fix(self, style, begin_pos, bytes.len(), corr);
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
