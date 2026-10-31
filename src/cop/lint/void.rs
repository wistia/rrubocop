use tree_sitter::Node;

use crate::cop::shared::{call_method_name, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/Void — literals/ops/vars in void context.
pub struct Void;

/// RuboCop `BINARY_OPERATORS` — excludes `<<`, `**`, `&&`, etc.
const VOID_OPS: &[&str] = &[
    "*", "/", "%", "+", "-", "==", "===", "!=", "<", ">", "<=", ">=", "<=>",
];

fn is_var_kind(kind: &str) -> bool {
    matches!(
        kind,
        "instance_variable" | "class_variable" | "global_variable"
    )
}

fn assigned_earlier(source: &SourceFile, stmts: &[Node<'_>], idx: usize, name: &str) -> bool {
    for stmt in &stmts[..idx] {
        if stmt.kind() != "assignment" {
            continue;
        }
        let Some(left) = stmt.child_by_field_name("left") else {
            continue;
        };
        if left.kind() == "identifier" && node_text(source, left) == name {
            return true;
        }
    }
    false
}

fn void_literal_msg(source: &SourceFile, stmt: Node<'_>) -> String {
    format!(
        "Literal `{}` used in void context.",
        node_text(source, stmt)
    )
}

fn void_named_msg(kind: &str, source: &SourceFile, stmt: Node<'_>) -> String {
    format!("{kind} `{}` used in void context.", node_text(source, stmt))
}

fn void_ident_msg(
    source: &SourceFile,
    stmts: &[Node<'_>],
    idx: usize,
    stmt: Node<'_>,
) -> Option<String> {
    let name = node_text(source, stmt);
    assigned_earlier(source, stmts, idx, &name)
        .then(|| format!("Variable `{name}` used in void context."))
}

fn void_binary_msg(source: &SourceFile, stmt: Node<'_>) -> Option<String> {
    let op = stmt
        .child_by_field_name("operator")
        .map(|o| node_text(source, o))
        .unwrap_or_default();
    VOID_OPS
        .contains(&op.as_str())
        .then(|| format!("Operator `{op}` used in void context."))
}

fn void_msg(source: &SourceFile, stmts: &[Node<'_>], idx: usize, stmt: Node<'_>) -> Option<String> {
    if let Some(msg) = void_literal_kind(source, stmt) {
        return Some(msg);
    }
    match stmt.kind() {
        "self" => Some("`self` used in void context.".to_string()),
        "constant" => Some(void_named_msg("Constant", source, stmt)),
        "scope_resolution" => Some(void_named_msg("Constant", source, stmt)),
        k if is_var_kind(k) => Some(void_named_msg("Variable", source, stmt)),
        "identifier" => void_ident_msg(source, stmts, idx, stmt),
        "binary" => void_binary_msg(source, stmt),
        _ => None,
    }
}

fn void_literal_kind(source: &SourceFile, stmt: Node<'_>) -> Option<String> {
    match stmt.kind() {
        "integer" | "float" | "string" | "simple_symbol" | "regex" | "true" | "false" | "nil" => {
            Some(void_literal_msg(source, stmt))
        }
        "array" | "hash" => entirely_literal(source, stmt).then(|| void_literal_msg(source, stmt)),
        _ => None,
    }
}

fn kids_all_literal(source: &SourceFile, node: Node<'_>) -> bool {
    let mut cur = node.walk();
    node.named_children(&mut cur)
        .all(|n| entirely_literal(source, n))
}

/// RuboCop `entirely_literal?` — `[deposit]` is not a void literal.
fn entirely_literal(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "integer" | "float" | "string" | "simple_symbol" | "regex" | "true" | "false" | "nil" => {
            true
        }
        "array" | "hash" | "pair" => kids_all_literal(source, node),
        "call" => {
            call_method_name(source, node) == Some(b"freeze")
                && node
                    .child_by_field_name("receiver")
                    .is_some_and(|r| entirely_literal(source, r))
        }
        _ => false,
    }
}

fn check_void_stmts(
    source: &SourceFile,
    stmts: &[Node<'_>],
    cop: &Void,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (idx, stmt) in stmts.iter().enumerate().take(stmts.len().saturating_sub(1)) {
        let Some(msg) = void_msg(source, stmts, idx, *stmt) else {
            continue;
        };
        let (line, col) = source.offset_to_line_col(stmt.start_byte());
        diagnostics.push(cop.diagnostic(source, line, col, msg));
    }
}

fn nonmutating_suggest(meth: &[u8]) -> Option<&'static str> {
    match meth {
        b"map" | b"collect" => Some("each"),
        b"reverse" => Some("reverse!"),
        b"sort" => Some("sort!"),
        _ => None,
    }
}

fn check_nonmutating(
    source: &SourceFile,
    stmts: &[Node<'_>],
    cop: &Void,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for stmt in stmts.iter().take(stmts.len().saturating_sub(1)) {
        if stmt.kind() != "call" {
            continue;
        }
        let Some(meth) = call_method_name(source, *stmt) else {
            continue;
        };
        let Some(sug) = nonmutating_suggest(meth) else {
            continue;
        };
        let m = String::from_utf8_lossy(meth);
        let (line, col) = source.offset_to_line_col(stmt.start_byte());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            col,
            format!("Method `#{m}` used in void context. Did you mean `#{sug}`?"),
        ));
    }
}

impl Cop for Void {
    fn name(&self) -> &'static str {
        "Lint/Void"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["body_statement", "block_body"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // tree-sitter puts rescue/else/ensure beside statements in body_statement;
        // RuboCop only checks the statement list (last stmt is not void).
        let mut cur = node.walk();
        let stmts: Vec<_> = node
            .named_children(&mut cur)
            .filter(|n| {
                !matches!(
                    n.kind(),
                    "rescue" | "else" | "ensure" | "comment" | "heredoc_body"
                )
            })
            .collect();
        check_void_stmts(source, &stmts, self, diagnostics);
        if config.get_bool("CheckForMethodsWithNoSideEffects", false) {
            check_nonmutating(source, &stmts, self, diagnostics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Void, "cops/lint/void");
}
