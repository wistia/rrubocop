//! Layout/EmptyLineAfterGuardClause.

use tree_sitter::Node;

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLineAfterGuardClause;

fn modifier_is_guard(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(body) = node.child_by_field_name("body") else {
        return false;
    };
    let k = body.kind();
    // Only `return`/`break`/`next`/`raise` … if/unless — not arbitrary modifiers.
    matches!(k, "return" | "break" | "next")
        || shared::call_method_name(source, body) == Some(b"raise")
        || (body.kind() == "identifier" && shared::node_bytes(source, body) == b"raise")
}

fn is_guard_node(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(node.kind(), "if_modifier" | "unless_modifier") && modifier_is_guard(source, node)
}

fn is_statement_level(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    // Exclude block/do bodies — Parser `begin` inside blocks is checked by RuboCop
    // for other cops, but EmptyLineAfterGuardClause only flags top-level-ish stmts.
    // Match RuboCop: guard is a direct child of the body that holds sequential stmts
    // outside of a block argument (parent of block_body is block/do_block).
    if matches!(parent.kind(), "block_body" | "block" | "do_block") {
        return false;
    }
    matches!(
        parent.kind(),
        "body_statement" | "begin" | "then" | "else" | "ensure" | "rescue" | "program"
    )
}

fn contains_guard_clause(source: &SourceFile, node: Node<'_>) -> bool {
    if is_guard_node(source, node) {
        return true;
    }
    let mut cur = node.walk();
    for c in node.named_children(&mut cur) {
        if matches!(c.kind(), "return" | "next" | "break") {
            return true;
        }
        if contains_guard_clause(source, c) {
            return true;
        }
    }
    false
}

fn stmt_siblings<'a>(parent: Node<'a>) -> Vec<Node<'a>> {
    let mut cur = parent.walk();
    parent
        .named_children(&mut cur)
        .filter(|n| !matches!(n.kind(), "comment" | "rescue" | "ensure" | "else"))
        .collect()
}

/// RuboCop `next_sibling_empty_or_guard_clause?`.
fn next_sibling_is_guard_if(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let kids = stmt_siblings(parent);
    let Some(idx) = kids.iter().position(|k| k.id() == node.id()) else {
        return false;
    };
    let Some(next) = kids.get(idx + 1) else {
        return true;
    };
    matches!(next.kind(), "if" | "unless") && contains_guard_clause(source, *next)
}

fn next_is_kw_or_comment(bytes: &[u8], ls: usize) -> bool {
    let mut i = ls;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    let rest = &bytes[i..];
    rest.starts_with(b"end")
        || rest.starts_with(b"rescue")
        || rest.starts_with(b"ensure")
        || rest.starts_with(b"else")
        || rest.starts_with(b"elsif")
        || rest.starts_with(b"when")
        || rest.starts_with(b"#")
}

fn line_starts_with_guard(source: &SourceFile, line: usize) -> bool {
    let Some(ls) = source.line_start(line) else {
        return false;
    };
    let bytes = source.as_bytes();
    let mut i = ls;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    let rest = &bytes[i..];
    rest.starts_with(b"return ")
        || rest.starts_with(b"return\t")
        || rest.starts_with(b"break ")
        || rest.starts_with(b"next ")
        || rest.starts_with(b"raise ")
        || rest.starts_with(b"raise(")
}

impl Cop for EmptyLineAfterGuardClause {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineAfterGuardClause"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["if_modifier", "unless_modifier"]
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
        if !is_guard_node(source, node) || !is_statement_level(node) {
            return;
        }
        let (end_line, _) = source.offset_to_line_col(node.end_byte().saturating_sub(1));
        let next = end_line + 1;
        if skip_empty_line_after(source, node, next) {
            return;
        }
        report_missing_blank(self, source, node, next, diagnostics, &mut corrections);
    }
}

/// RuboCop reports on the guard node; autocorrect inserts a blank before `next`.
fn report_missing_blank(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    next: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, column) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        column,
        "Add empty line after guard clause.".into(),
    );
    if let Some(corr) = corrections {
        if let Some(offset) = source.line_start(next) {
            corr.push(Correction {
                start: offset,
                end: offset,
                replacement: "\n".into(),
                cop_name: cop.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

fn skip_empty_line_after(source: &SourceFile, node: Node<'_>, next: usize) -> bool {
    if source.line_start(next).is_none() || shared::line_blank(source, next) {
        return true;
    }
    if source
        .line_start(next)
        .is_some_and(|ls| next_is_kw_or_comment(source.as_bytes(), ls))
    {
        return true;
    }
    // Consecutive guards / next sibling `if` with a guard — no blank.
    line_starts_with_guard(source, next) || next_sibling_is_guard_if(source, node)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        EmptyLineAfterGuardClause,
        "cops/layout/empty_line_after_guard_clause"
    );
}
