//! Layout/SpaceInsidePercentLiteralDelimiters.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsidePercentLiteralDelimiters;

fn matching_close(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        c => c,
    }
}

fn percent_type(text: &[u8]) -> Option<u8> {
    if !text.starts_with(b"%") || text.len() < 3 {
        return None;
    }
    let b = text[1];
    if b.is_ascii_alphanumeric() {
        Some(b)
    } else {
        None
    }
}

fn find_open(text: &[u8]) -> Option<usize> {
    if !text.starts_with(b"%") || text.len() < 4 {
        return None;
    }
    Some(
        text.iter()
            .position(|&b| !b.is_ascii_alphanumeric() && b != b'%')
            .unwrap_or(1),
    )
}

fn percent_inner(bytes: &[u8], node: Node<'_>) -> Option<(usize, usize)> {
    let text = &bytes[node.start_byte()..node.end_byte()];
    let oi = find_open(text)?;
    let close_rel = text.iter().rposition(|&b| b == matching_close(text[oi]))?;
    let inner_s = node.start_byte() + oi + 1;
    let inner_e = node.start_byte() + close_rel;
    (inner_e > inner_s).then_some((inner_s, inner_e))
}

fn strip_sides(
    corr: &mut Vec<Correction>,
    cop_name: &'static str,
    bytes: &[u8],
    inner_s: usize,
    inner_e: usize,
    sp_a: bool,
    sp_b: bool,
) {
    if sp_a {
        let mut e = inner_s;
        while e < inner_e && matches!(bytes[e], b' ' | b'\t') {
            e += 1;
        }
        corr.push(Correction {
            start: inner_s,
            end: e,
            replacement: String::new(),
            cop_name,
            cop_index: 0,
            owner: None,
        });
    }
    if sp_b {
        let mut s = inner_e;
        while s > inner_s && matches!(bytes[s - 1], b' ' | b'\t') {
            s -= 1;
        }
        corr.push(Correction {
            start: s,
            end: inner_e,
            replacement: String::new(),
            cop_name,
            cop_index: 0,
            owner: None,
        });
    }
}

fn report_spaces(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    node: Node<'_>,
    inner_s: usize,
    inner_e: usize,
    sp_a: bool,
    sp_b: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (l, c) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        l,
        c,
        "Do not use spaces inside percent literal delimiters.".into(),
    );
    if let Some(corr) = corrections {
        strip_sides(corr, cop.name(), bytes, inner_s, inner_e, sp_a, sp_b);
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

impl Cop for SpaceInsidePercentLiteralDelimiters {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsidePercentLiteralDelimiters"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        // RuboCop only checks %i/%I/%w/%W/%x — not %() / %Q / %r / %s strings.
        &["string_array", "symbol_array", "%w", "%W", "%i", "%I", "%x"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = config;
        let bytes = source.as_bytes();
        let text = &bytes[node.start_byte()..node.end_byte()];
        // Guard: only %w/%W/%i/%I/%x (case-insensitive type letter).
        if !matches!(
            percent_type(text),
            Some(b'w' | b'W' | b'i' | b'I' | b'x' | b'X')
        ) {
            return;
        }
        let Some((inner_s, inner_e)) = percent_inner(bytes, node) else {
            return;
        };
        let sp_a = matches!(bytes.get(inner_s), Some(b' ') | Some(b'\t'));
        let sp_b = matches!(bytes.get(inner_e - 1), Some(b' ') | Some(b'\t'));
        let inner = &bytes[inner_s..inner_e];
        // RuboCop `add_offenses_for_unnecessary_spaces` only runs on single-line literals.
        if inner.contains(&b'\n') {
            return;
        }
        if sp_a || sp_b {
            report_spaces(
                self,
                source,
                bytes,
                node,
                inner_s,
                inner_e,
                sp_a,
                sp_b,
                diagnostics,
                &mut corrections,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SpaceInsidePercentLiteralDelimiters,
        "cops/layout/space_inside_percent_literal_delimiters"
    );
}
