//! Layout/HeredocIndentation.

use tree_sitter::Node;

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HeredocIndentation;

fn skip_name(bytes: &[u8], mut j: usize) -> usize {
    while j < bytes.len()
        && (bytes[j].is_ascii_alphanumeric() || matches!(bytes[j], b'_' | b'\'' | b'"' | b'`'))
    {
        j += 1;
    }
    j
}

fn try_opener_at(bytes: &[u8], i: usize) -> Option<(usize, usize, bool)> {
    if bytes[i] != b'<' || i == 0 || bytes[i - 1] != b'<' {
        return None;
    }
    let start = i - 1;
    let mut j = i + 1;
    let mut squiggly = false;
    if matches!(bytes.get(j), Some(&b'-') | Some(&b'~')) {
        squiggly = bytes[j] == b'~';
        j += 1;
    }
    Some((start, skip_name(bytes, j), squiggly))
}

fn find_opener(source: &SourceFile, body: Node<'_>) -> Option<(usize, usize, bool)> {
    let bytes = source.as_bytes();
    let mut i = body.start_byte();
    while i > 0 {
        i -= 1;
        if let Some(found) = try_opener_at(bytes, i) {
            return Some(found);
        }
    }
    None
}

fn body_indents(body: &[u8]) -> Vec<usize> {
    let lines: Vec<&[u8]> = body.split(|&b| b == b'\n').collect();
    lines[..lines.len().saturating_sub(1)]
        .iter()
        .filter(|l| !l.is_empty())
        .map(|line| {
            line.iter()
                .take_while(|&&b| b == b' ' || b == b'\t')
                .count()
        })
        .collect()
}

fn opener_info(source: &SourceFile, node: Node<'_>) -> (Option<(usize, usize, bool)>, bool, usize) {
    let opener = find_opener(source, node);
    (
        opener,
        opener.is_some_and(|(_, _, s)| !s),
        opener
            .map(|(start, _, _)| shared::line_indent(source, start))
            .unwrap_or(0),
    )
}

fn has_offense(
    indents: &[usize],
    width: usize,
    needs_squiggly: bool,
    opener_indent: usize,
) -> bool {
    let Some(&min_i) = indents.iter().min() else {
        return false;
    };
    if needs_squiggly {
        min_i == 0
    } else {
        // `<<~`: only the least-indented body line vs opener + width (extra
        // indent inside the content is allowed).
        min_i != opener_indent + width
    }
}

fn fix_to_squiggly(
    cop: &dyn Cop,
    bytes: &[u8],
    start: usize,
    end: usize,
    corr: &mut Vec<Correction>,
) {
    let op = &bytes[start..end];
    let rest = if op.starts_with(b"<<-") {
        &op[3..]
    } else {
        &op[2..]
    };
    corr.push(Correction {
        start,
        end,
        replacement: format!("<<~{}", String::from_utf8_lossy(rest)),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
}

fn fix_body_indents(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    lines: &[&[u8]],
    indents: &[usize],
    start: usize,
    width: usize,
    corr: &mut Vec<Correction>,
) {
    let target = shared::line_indent(source, start) + width;
    let mut off = node.start_byte();
    let mut ci = 0usize;
    for line_bytes in &lines[..lines.len().saturating_sub(1)] {
        if line_bytes.is_empty() {
            off += 1;
            continue;
        }
        let ind = indents[ci];
        ci += 1;
        if ind != target {
            corr.push(Correction {
                start: off,
                end: off + ind,
                replacement: " ".repeat(target),
                cop_name: cop.name(),
                cop_index: ci,
                owner: None,
            });
        }
        off += line_bytes.len() + 1;
    }
}

fn apply_fix(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    lines: &[&[u8]],
    indents: &[usize],
    opener: (usize, usize, bool),
    width: usize,
    corr: &mut Vec<Correction>,
) {
    let (start, end, squiggly) = opener;
    if squiggly {
        fix_body_indents(cop, source, node, lines, indents, start, width, corr);
    } else {
        fix_to_squiggly(cop, source.as_bytes(), start, end, corr);
    }
}

fn offense_msg(width: usize, needs_squiggly: bool) -> String {
    if needs_squiggly {
        format!("Use {width} spaces for indentation in a heredoc by using `<<~` instead of `<<`.")
    } else {
        format!("Use {width} spaces for indentation in a heredoc.")
    }
}

fn maybe_report(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    lines: &[&[u8]],
    indents: &[usize],
    opener: Option<(usize, usize, bool)>,
    width: usize,
    needs_squiggly: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (l, _) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(source, l, 0, offense_msg(width, needs_squiggly));
    if let (Some(corr), Some(op)) = (corrections.as_mut(), opener) {
        apply_fix(cop, source, node, lines, indents, op, width, corr);
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

impl Cop for HeredocIndentation {
    fn name(&self) -> &'static str {
        "Layout/HeredocIndentation"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["heredoc_body"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let width = config.get_usize("IndentationWidth", 2);
        let body = shared::node_bytes(source, node);
        let lines: Vec<&[u8]> = body.split(|&b| b == b'\n').collect();
        if lines.len() < 2 {
            return;
        }
        let indents = body_indents(body);
        let (opener, needs_squiggly, opener_indent) = opener_info(source, node);
        if !has_offense(&indents, width, needs_squiggly, opener_indent) {
            return;
        }
        maybe_report(
            self,
            source,
            node,
            &lines,
            &indents,
            opener,
            width,
            needs_squiggly,
            diagnostics,
            &mut corrections,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HeredocIndentation, "cops/layout/heredoc_indentation");
}
