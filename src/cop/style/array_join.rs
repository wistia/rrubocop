//! Style/ArrayJoin
use crate::cop::shared::node_text;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ArrayJoin;

impl Cop for ArrayJoin {
    fn name(&self) -> &'static str {
        "Style/ArrayJoin"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["binary"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _c: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((op, left, right)) = detect(node) else {
            return;
        };
        report(
            self,
            source,
            node,
            op,
            left,
            right,
            diagnostics,
            &mut corrections,
        );
    }
}

fn detect(node: Node<'_>) -> Option<(Node<'_>, Node<'_>, Node<'_>)> {
    Some((star_op(node)?, left_operand(node)?, right_string(node)?))
}

fn report(
    cop: &ArrayJoin,
    source: &SourceFile,
    node: Node<'_>,
    op: Node<'_>,
    left: Node<'_>,
    right: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (l, c) = source.offset_to_line_col(op.start_byte());
    let mut diag = cop.diagnostic(source, l, c, "Favor `Array#join` over `Array#*`.".into());
    if let Some(corr) = corrections.as_mut() {
        let left_src = node_text(source, left);
        let sep = node_text(source, right);
        corr.push(Correction {
            start: node.start_byte(),
            end: node.end_byte(),
            replacement: format!("{left_src}.join({sep})"),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn star_op(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|ch| !ch.is_named() && ch.kind() == "*")
}

fn right_string(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .collect::<Vec<_>>()
        .iter()
        .rev()
        .find(|ch| ch.is_named())
        .copied()
        .filter(|r| r.kind() == "string")
}

fn left_operand(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|ch| ch.is_named())
        .filter(|n| matches!(n.kind(), "array" | "percent_literal"))
}
