//! Shared space-inside-delimiter helpers for Layout/SpaceInside* cops.

use crate::cop::Cop;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DelimSpace {
    pub inner_s: usize,
    pub inner_e: usize,
    pub nl_a: bool,
    pub nl_b: bool,
    pub sp_a: bool,
    pub sp_b: bool,
}

pub fn scan_inner(bytes: &[u8], inner_s: usize, inner_e: usize) -> Option<DelimSpace> {
    if inner_e < inner_s {
        return None;
    }
    let after = bytes.get(inner_s).copied();
    let before = if inner_e > inner_s {
        bytes.get(inner_e - 1).copied()
    } else {
        None
    };
    Some(DelimSpace {
        inner_s,
        inner_e,
        nl_a: matches!(after, Some(b'\n') | Some(b'\r')),
        nl_b: matches!(before, Some(b'\n') | Some(b'\r')),
        sp_a: matches!(after, Some(b' ') | Some(b'\t')),
        sp_b: matches!(before, Some(b' ') | Some(b'\t')),
    })
}

pub fn is_blank_inner(bytes: &[u8], inner_s: usize, inner_e: usize) -> bool {
    inner_s == inner_e
        || bytes[inner_s..inner_e]
            .iter()
            .all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
}

fn push_corr(
    corr: &mut Option<&mut Vec<Correction>>,
    cop_name: &'static str,
    start: usize,
    end: usize,
    replacement: String,
) -> bool {
    if let Some(c) = corr {
        c.push(Correction {
            start,
            end,
            replacement,
            cop_name,
            cop_index: 0,
            owner: None,
        });
        true
    } else {
        false
    }
}

pub fn report_at(
    cop: &dyn Cop,
    source: &SourceFile,
    off: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    start: usize,
    end: usize,
    replacement: String,
    correctable: Option<bool>,
) {
    let (line, col) = source.offset_to_line_col(off);
    let mut diag = cop.diagnostic(source, line, col, msg);
    if let Some(c) = correctable {
        diag.correctable = c;
    }
    if diag.correctable && push_corr(corrections, cop.name(), start, end, replacement) {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

pub fn add_space_after(
    cop: &dyn Cop,
    source: &SourceFile,
    d: &DelimSpace,
    report_off: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if d.nl_a || d.sp_a {
        return;
    }
    report_at(
        cop,
        source,
        report_off,
        msg,
        diagnostics,
        corrections,
        d.inner_s,
        d.inner_s,
        " ".into(),
        None,
    );
}

pub fn add_space_before(
    cop: &dyn Cop,
    source: &SourceFile,
    d: &DelimSpace,
    report_off: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if d.nl_b || d.sp_b {
        return;
    }
    report_at(
        cop,
        source,
        report_off,
        msg,
        diagnostics,
        corrections,
        d.inner_e,
        d.inner_e,
        " ".into(),
        None,
    );
}

pub fn strip_space_after(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    d: &DelimSpace,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    correctable: &mut bool,
) {
    if !(d.sp_a && !d.nl_a) {
        return;
    }
    let mut e = d.inner_s;
    while e < d.inner_e && matches!(bytes[e], b' ' | b'\t') {
        e += 1;
    }
    let can = *correctable && cop.supports_autocorrect();
    report_at(
        cop,
        source,
        d.inner_s,
        msg,
        diagnostics,
        corrections,
        d.inner_s,
        e,
        String::new(),
        Some(can),
    );
    if can {
        *correctable = false;
    }
}

pub fn strip_space_before(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    d: &DelimSpace,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    correctable: &mut bool,
) {
    if !(d.sp_b && !d.nl_b) {
        return;
    }
    // `]\n` alone on a line: leading indent is not "space inside brackets".
    if delim_first_on_line(bytes, d.inner_e) {
        return;
    }
    let mut s = d.inner_e;
    while s > d.inner_s && matches!(bytes[s - 1], b' ' | b'\t') {
        s -= 1;
    }
    let can = *correctable && cop.supports_autocorrect();
    report_at(
        cop,
        source,
        d.inner_e.saturating_sub(1),
        msg,
        diagnostics,
        corrections,
        s,
        d.inner_e,
        String::new(),
        Some(can),
    );
    if can {
        *correctable = false;
    }
}

fn delim_first_on_line(bytes: &[u8], pos: usize) -> bool {
    let mut i = pos;
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b'\n' => return true,
            b' ' | b'\t' | b'\r' => {}
            _ => return false,
        }
    }
    true
}

/// RuboCop `ignore_node`: only the first strip offense per delimiter is correctable.
pub fn strip_spaces(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    d: &DelimSpace,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut correctable = true;
    strip_space_after(
        cop,
        source,
        bytes,
        d,
        msg.clone(),
        diagnostics,
        corrections,
        &mut correctable,
    );
    strip_space_before(
        cop,
        source,
        bytes,
        d,
        msg,
        diagnostics,
        corrections,
        &mut correctable,
    );
}

/// Enforce spaces; after-open reports at `open_off`, before-close at `close_off` when adding.
pub fn enforce_spaces(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    d: &DelimSpace,
    want: bool,
    open_off: usize,
    close_off: usize,
    msg: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if want {
        add_space_after(
            cop,
            source,
            d,
            open_off,
            msg.into(),
            diagnostics,
            corrections,
        );
        add_space_before(
            cop,
            source,
            d,
            close_off,
            msg.into(),
            diagnostics,
            corrections,
        );
    } else {
        strip_spaces(cop, source, bytes, d, msg.into(), diagnostics, corrections);
    }
}
