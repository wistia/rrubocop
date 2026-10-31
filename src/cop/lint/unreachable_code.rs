use tree_sitter::Node;

use crate::cop::shared::{call_method_name, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/UnreachableCode — statements after return/raise/next/break.
pub struct UnreachableCode;

fn is_terminator(source: &crate::parse::source::SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "return" | "next" | "break" | "retry" | "redo" => true,
        "call" => {
            if node.child_by_field_name("receiver").is_some() {
                return false;
            }
            let meth = call_method_name(source, node).unwrap_or(b"");
            matches!(meth, b"raise" | b"fail" | b"throw" | b"exit" | b"abort")
        }
        "identifier" => matches!(node_bytes(source, node), b"raise" | b"fail" | b"throw"),
        _ => false,
    }
}

fn check_stmts(
    source: &SourceFile,
    stmts: &[Node<'_>],
    cop: &UnreachableCode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut terminated = false;
    for stmt in stmts {
        if terminated {
            let (line, col) = source.offset_to_line_col(stmt.start_byte());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                col,
                "Unreachable code detected.".to_string(),
            ));
            return;
        }
        if is_terminator(source, *stmt) {
            terminated = true;
        }
    }
}

impl Cop for UnreachableCode {
    fn name(&self) -> &'static str {
        "Lint/UnreachableCode"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        // Method/`begin` bodies (Parser begin). Keep block_body for do-blocks
        // (RuboCop flags `next`+code there unless disabled).
        &["body_statement", "block_body", "begin", "then", "else"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut cur = node.walk();
        // Skip rescue/else/ensure siblings (same as Lint/Void) — they are not
        // unreachable code after return/raise in the statement list.
        check_stmts(
            source,
            &node
                .named_children(&mut cur)
                .filter(|n| {
                    !matches!(
                        n.kind(),
                        "rescue" | "else" | "ensure" | "comment" | "heredoc_body"
                    )
                })
                .collect::<Vec<_>>(),
            self,
            diagnostics,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnreachableCode, "cops/lint/unreachable_code");
}
