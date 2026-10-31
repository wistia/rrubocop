//! Style/EmptyElse — empty else branch.

use tree_sitter::Node;

use crate::cop::shared::child_kind;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyElse;

impl Cop for EmptyElse {
    fn name(&self) -> &'static str {
        "Style/EmptyElse"
    }

    fn redundant_disable_audit(&self) -> bool {
        false
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["if", "unless", "case"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(else_n) = child_kind(node, "else") else {
            return;
        };
        if !is_empty_else(else_n) {
            return;
        }
        report(self, source, node, else_n, diagnostics, &mut corrections);
    }
}

fn is_empty_else(else_n: Node<'_>) -> bool {
    let mut cur = else_n.walk();
    let named: Vec<_> = else_n.named_children(&mut cur).collect();
    if named.is_empty() {
        return true;
    }
    if named.len() == 1 && named[0].kind() == "nil" {
        return true;
    }
    if named.len() == 1 && named[0].kind() == "then" {
        return named[0]
            .named_children(&mut named[0].walk())
            .next()
            .is_none();
    }
    false
}

fn report(
    cop: &EmptyElse,
    source: &SourceFile,
    node: Node<'_>,
    else_n: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(else_n.start_byte());
    let mut diag = cop.diagnostic(source, line, col, "Redundant `else`-clause.".to_string());
    if let Some(corr) = corrections {
        push_remove(cop, source, node, else_n, corr, &mut diag);
    }
    diagnostics.push(diag);
}

fn push_remove(
    cop: &EmptyElse,
    source: &SourceFile,
    node: Node<'_>,
    else_n: Node<'_>,
    corr: &mut Vec<Correction>,
    diag: &mut Diagnostic,
) {
    let Some(end_n) = child_kind(node, "end") else {
        return;
    };
    let src = source.as_bytes();
    let remove_start = line_start_before(src, else_n.start_byte());
    let remove_end = {
        let pos = line_start_before(src, end_n.start_byte());
        if pos > 0 { pos - 1 } else { end_n.start_byte() }
    };
    let remove_start = if remove_start > 0 {
        remove_start - 1
    } else {
        0
    };
    corr.push(Correction {
        start: remove_start,
        end: remove_end,
        replacement: String::new(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    diag.corrected = true;
}

fn line_start_before(src: &[u8], mut pos: usize) -> usize {
    while pos > 0 && src[pos - 1] != b'\n' {
        pos -= 1;
    }
    pos
}
