//! Shared `end` keyword alignment for Layout/*EndAlignment cops.

use tree_sitter::Node;

use crate::cop::Cop;
use crate::cop::shared;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

fn apply_fix(
    source: &SourceFile,
    end_kw: Node<'_>,
    el: usize,
    base_col: usize,
    cop_name: &'static str,
    corr: &mut Vec<Correction>,
) -> bool {
    let Some(ls) = source.line_start(el) else {
        return false;
    };
    corr.push(Correction {
        start: ls,
        end: ls + shared::line_indent(source, end_kw.start_byte()),
        replacement: " ".repeat(base_col),
        cop_name,
        cop_index: 0,
        owner: None,
    });
    true
}

fn report_misaligned_end(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    end_kw: Node<'_>,
    base_name: &str,
    base_col: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (el, ec) = source.offset_to_line_col(end_kw.start_byte());
    let (bl, bc) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        el,
        ec,
        format!("`end` at {el}, {ec} is not aligned with `{base_name}` at {bl}, {bc}."),
    );
    if let Some(corr) = corrections {
        if apply_fix(source, end_kw, el, base_col, cop.name(), corr) {
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

/// Column of the assignment/`<<` receiver when `kw_offset` sits on its RHS
/// (e.g. `x = if`, `@x ||= if`, `buf << if`). Mirrors nitrocop / RuboCop
/// variable-alignment context: first non-whitespace on the assignment line.
pub fn assignment_context_base_col(source: &SourceFile, kw_offset: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let before = &bytes[line_start_before(bytes, kw_offset)..kw_offset];
    first_non_ws_if_assign(before).or_else(|| first_non_ws_if_shovel(before))
}

fn line_start_before(bytes: &[u8], offset: usize) -> usize {
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    line_start
}

fn first_non_ws(before: &[u8]) -> Option<usize> {
    before.iter().position(|&b| b != b' ' && b != b'\t')
}

fn first_non_ws_if_assign(before: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < before.len() {
        if before[i] != b'=' {
            i += 1;
            continue;
        }
        let next = before.get(i + 1).copied().unwrap_or(b' ');
        if matches!(next, b'=' | b'~' | b'>') {
            i += 2;
            continue;
        }
        // Skip !=, <=, >=, == (second `=` already handled above).
        if i > 0 && matches!(before[i - 1], b'!' | b'<' | b'>' | b'=') {
            i += 1;
            continue;
        }
        // Bare `=` / `||=` / `+=` / … — align with line's first non-ws.
        return first_non_ws(before);
    }
    None
}

fn first_non_ws_if_shovel(before: &[u8]) -> Option<usize> {
    let mut j = 0;
    while j + 1 < before.len() {
        if before[j] == b'<' && before[j + 1] == b'<' {
            let next = before.get(j + 2).copied().unwrap_or(b' ');
            if matches!(next, b'=' | b'~' | b'-') {
                j += 3;
                continue;
            }
            return first_non_ws(before);
        }
        j += 1;
    }
    None
}

fn align_col(source: &SourceFile, node: Node<'_>, style: &str) -> usize {
    match style {
        "start_of_line" => shared::line_indent(source, node.start_byte()),
        // RuboCop: variable style falls back to keyword unless same-line assignment /
        // `case` method-argument RHS (`on_case` + `argument?`).
        "variable" => same_line_assign_col(source, node)
            .or_else(|| assignment_context_base_col(source, node.start_byte()))
            .or_else(|| same_line_case_arg_col(source, node))
            .unwrap_or_else(|| shared::node_col(source, node)),
        _ => shared::node_col(source, node),
    }
}

/// RuboCop `Layout/EndAlignment` `on_case` when `node.argument?`: same-line `case`
/// as a call arg aligns `end` with the call (`foo(case` / `test case`), not `case`.
fn same_line_case_arg_col(source: &SourceFile, node: Node<'_>) -> Option<usize> {
    if node.kind() != "case" {
        return None;
    }
    let args = node
        .parent()
        .filter(|p| matches!(p.kind(), "argument_list" | "command_argument_list"))?;
    let call = args
        .parent()
        .filter(|p| matches!(p.kind(), "call" | "command" | "command_call"))?;
    (shared::node_line(source, call) == shared::node_line(source, node))
        .then(|| shared::node_col(source, call))
}

/// Column of a same-line `=` / `||=` / `+=` assignment that has `node` on its RHS
/// (RuboCop `CheckAssignment` / variable EndAlignment).
pub fn same_line_assign_col(source: &SourceFile, node: Node<'_>) -> Option<usize> {
    let kw_line = shared::node_line(source, node);
    let mut p = node.parent();
    while let Some(cur) = p {
        if matches!(cur.kind(), "assignment" | "operator_assignment") {
            let same = shared::node_line(source, cur) == kw_line;
            return same.then(|| shared::node_col(source, cur));
        }
        if matches!(
            cur.kind(),
            "program" | "method" | "singleton_method" | "class" | "module"
        ) {
            break;
        }
        p = cur.parent();
    }
    None
}

/// Align `end` using `EnforcedStyleAlignWith` (`keyword` / `variable` / `start_of_line`).
pub fn check_end(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    base_name: &str,
    style: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some(end_kw) = shared::end_keyword(node) else {
        return;
    };
    // Same-line `end` (e.g. `def foo; end`) is not checked by RuboCop.
    if shared::node_line(source, end_kw) == shared::node_line(source, node) {
        return;
    }
    let base_col = align_col(source, node, style);
    if shared::node_col(source, end_kw) == base_col {
        return;
    }
    report_misaligned_end(
        cop,
        source,
        node,
        end_kw,
        base_name,
        base_col,
        diagnostics,
        corrections,
    );
}
