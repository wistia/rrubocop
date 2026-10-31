//! Style/NegatedWhile.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NegatedWhile;

impl Cop for NegatedWhile {
    fn name(&self) -> &'static str {
        "Style/NegatedWhile"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["while", "while_modifier"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(cond) = node.child_by_field_name("condition") else {
            return;
        };
        let Some(inner) = negated_operand(source, cond) else {
            return;
        };
        report(
            self,
            source,
            node,
            cond,
            inner,
            diagnostics,
            &mut corrections,
        );
    }
}

fn report(
    cop: &NegatedWhile,
    source: &SourceFile,
    node: Node<'_>,
    cond: Node<'_>,
    inner: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Favor `until` over `while` for negative conditions.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        push_until(cop, source, node, cond, inner, corr, &mut diag);
    }
    diagnostics.push(diag);
}

fn push_until(
    cop: &NegatedWhile,
    source: &SourceFile,
    node: Node<'_>,
    cond: Node<'_>,
    inner: Node<'_>,
    corr: &mut Vec<Correction>,
    diag: &mut Diagnostic,
) {
    let Some(while_kw) = keyword_child(source, node, b"while") else {
        return;
    };
    corr.push(Correction {
        start: while_kw.start_byte(),
        end: while_kw.end_byte(),
        replacement: "until".to_string(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    let inner_src = String::from_utf8_lossy(node_bytes(source, inner)).into_owned();
    let replacement = if cond.start_byte() == while_kw.end_byte() {
        format!(" {inner_src}")
    } else {
        inner_src
    };
    corr.push(Correction {
        start: cond.start_byte(),
        end: cond.end_byte(),
        replacement,
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    diag.corrected = true;
}

fn keyword_child<'a>(source: &SourceFile, node: Node<'a>, kw: &[u8]) -> Option<Node<'a>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|c| node_bytes(source, *c) == kw)
}

fn negated_operand<'a>(source: &SourceFile, cond: Node<'a>) -> Option<Node<'a>> {
    if cond.kind() != "unary" {
        return None;
    }
    let mut cur = cond.walk();
    if !cond.children(&mut cur).any(|c| {
        let t = node_bytes(source, c);
        t == b"!" || t == b"not"
    }) {
        return None;
    }
    cond.child_by_field_name("operand")
}
