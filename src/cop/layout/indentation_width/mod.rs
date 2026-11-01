//! Layout/IndentationWidth.

mod body;
mod cont;

use tree_sitter::Tree;

use crate::cop::layout::report;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct IndentationWidth;

fn line_indent(line: &[u8]) -> Option<usize> {
    if line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r') {
        return None;
    }
    let indent = line
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count();
    let rest = &line[indent..];
    if rest.starts_with(b"#")
        || rest.starts_with(b".")
        || rest.starts_with(b"&.")
        || is_branch_keyword(rest)
    {
        None
    } else {
        Some(indent)
    }
}

fn is_branch_keyword(rest: &[u8]) -> bool {
    // Bare `rescue`/`else`/`ensure` only — `rescue Foo,` is a continued
    // exception list and must keep a real indent for step checks.
    let rest = trim_ascii_end(rest);
    matches!(rest, b"else" | b"rescue" | b"ensure")
        || rest.starts_with(b"when ")
        || rest.starts_with(b"when(")
        || rest.starts_with(b"in ")
        || rest.starts_with(b"in(")
        || rest.starts_with(b"elsif ")
        || rest.starts_with(b"elsif(")
}

fn trim_ascii_end(code: &[u8]) -> &[u8] {
    let mut end = code.len();
    while end > 0 && matches!(code[end - 1], b' ' | b'\t' | b'\r') {
        end -= 1;
    }
    &code[..end]
}

fn expected_indent(indent: usize, prev: usize, width: usize) -> usize {
    if indent > prev {
        prev + width
    } else if prev >= width {
        prev - width
    } else {
        0
    }
}

fn bad_step(indent: usize, prev: usize, width: usize) -> bool {
    let diff = indent.abs_diff(prev);
    diff != 0 && width != 0 && diff % width != 0 && indent > 0
}

fn report_width(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    line_no: usize,
    indent: usize,
    prev: usize,
    width: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let off = source.line_start(line_no).unwrap_or(0);
    if code_map.covers(off + indent) {
        return;
    }
    let diff = indent.abs_diff(prev);
    report::report_fix(
        cop,
        source,
        off,
        format!("Use {width} (not {diff}) spaces for indentation."),
        diagnostics,
        corrections,
        off,
        off + indent,
        " ".repeat(expected_indent(indent, prev, width)),
    );
}

fn is_end_like(line: &[u8]) -> bool {
    let rest = trim_ascii_end(
        line.iter()
            .position(|&b| b != b' ' && b != b'\t')
            .map(|i| &line[i..])
            .unwrap_or(line),
    );
    matches!(rest, b"end" | b"}" | b"]" | b")")
        || rest.starts_with(b"end ")
        || rest.starts_with(b"end;")
}

fn end_relative_ok(source: &SourceFile, line_no: usize, indent: usize, width: usize) -> bool {
    // RuboCop `on_block`: body indent is checked against `end`, not `do`.
    // When `end` is itself mis-indented, body at end_col+width is accepted.
    for later in source.lines().enumerate().skip(line_no) {
        let (idx, line) = later;
        let Some(end_indent) = line_indent(line) else {
            continue;
        };
        if !is_end_like(line) {
            // Another indent level before end — not a direct block body match.
            if end_indent < indent {
                return false;
            }
            continue;
        }
        let _ = idx;
        return indent == end_indent.saturating_add(width);
    }
    false
}

fn check_step_from_prev(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    line_no: usize,
    indent: usize,
    prev: usize,
    width: usize,
    line: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if !bad_step(indent, prev, width) {
        return;
    }
    // `end` / closers: RuboCop EndAlignment owns their column; IndentationWidth
    // checks body against the closer instead.
    if is_end_like(line) {
        return;
    }
    if end_relative_ok(source, line_no, indent, width) {
        return;
    }
    report_width(
        cop,
        source,
        code_map,
        line_no,
        indent,
        prev,
        width,
        diagnostics,
        corrections,
    );
}

fn scan_file_indents(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    width: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut state = ScanState::default();
    for (i, line) in source.lines().enumerate() {
        scan_line(
            cop,
            source,
            code_map,
            width,
            i,
            line,
            &mut state,
            diagnostics,
            corrections,
        );
    }
}

struct ScanState<'a> {
    prev_indent: Option<usize>,
    prev_line: &'a [u8],
    cont_base: Option<usize>,
}

impl<'a> Default for ScanState<'a> {
    fn default() -> Self {
        Self {
            prev_indent: None,
            prev_line: b"",
            cont_base: None,
        }
    }
}

fn next_code_indent(source: &SourceFile, after: usize) -> Option<usize> {
    source.lines().skip(after + 1).find_map(|line| {
        if line.iter().all(|&b| matches!(b, b' ' | b'\t' | b'\r')) {
            return None;
        }
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        (!line[indent..].starts_with(b"#")).then_some(indent)
    })
}

fn scan_line<'a>(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    width: usize,
    i: usize,
    line: &'a [u8],
    state: &mut ScanState<'a>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if let Some(indent) = cont::deeper_dot_block(line, next_code_indent(source, i)) {
        state.prev_indent = Some(indent);
        state.prev_line = line;
        return;
    }
    let Some(indent) = line_indent(line) else {
        return;
    };
    if continue_or_check(
        cop,
        source,
        code_map,
        width,
        i,
        line,
        indent,
        state,
        diagnostics,
        corrections,
    ) {
        return;
    }
    if code_map.covers(source.line_start(i + 1).unwrap_or(0) + indent) {
        state.prev_line = line;
        return;
    }
    state.prev_indent = Some(indent);
    state.prev_line = line;
}

fn continue_or_check<'a>(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    width: usize,
    i: usize,
    line: &'a [u8],
    indent: usize,
    state: &mut ScanState<'a>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) -> bool {
    let Some(prev) = state.prev_indent else {
        return false;
    };
    if cont::aligned_continuation(indent, prev, state.prev_line, &mut state.cont_base) {
        state.prev_line = line;
        return true;
    }
    check_unless_after_end(
        cop,
        source,
        code_map,
        i + 1,
        indent,
        prev,
        width,
        line,
        state.prev_line,
        diagnostics,
        corrections,
    );
    false
}

fn check_unless_after_end(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    line_no: usize,
    indent: usize,
    prev: usize,
    width: usize,
    line: &[u8],
    prev_line: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if is_end_like(prev_line) {
        return;
    }
    check_step_from_prev(
        cop,
        source,
        code_map,
        line_no,
        indent,
        prev,
        width,
        line,
        diagnostics,
        corrections,
    );
}

impl Cop for IndentationWidth {
    fn name(&self) -> &'static str {
        "Layout/IndentationWidth"
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
        corrections: Option<&mut Vec<Correction>>,
    ) {
        let width = config.get_usize("Width", 2);
        body::check(
            self,
            source,
            tree.root_node(),
            code_map,
            config,
            diagnostics,
            corrections,
            |diags, corr| scan_file_indents(self, source, code_map, width, diags, corr),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IndentationWidth, "cops/layout/indentation_width");
}
