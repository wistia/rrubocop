//! Style/NegatedIf.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NegatedIf;

impl Cop for NegatedIf {
    fn name(&self) -> &'static str {
        "Style/NegatedIf"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["if", "if_modifier"]
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
        // RuboCop: only ifs without else/elsif (cannot convert to unless).
        if node.kind() == "if" {
            let mut cur = node.walk();
            if node
                .named_children(&mut cur)
                .any(|c| matches!(c.kind(), "else" | "elsif"))
            {
                return;
            }
        }
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
    cop: &NegatedIf,
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
        "Favor `unless` over `if` for negative conditions.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        push_unless(cop, source, node, cond, inner, corr, &mut diag);
    }
    diagnostics.push(diag);
}

fn push_unless(
    cop: &NegatedIf,
    source: &SourceFile,
    node: Node<'_>,
    cond: Node<'_>,
    inner: Node<'_>,
    corr: &mut Vec<Correction>,
    diag: &mut Diagnostic,
) {
    let Some(if_kw) = keyword_child(source, node, b"if") else {
        return;
    };
    corr.push(Correction {
        start: if_kw.start_byte(),
        end: if_kw.end_byte(),
        replacement: "unless".to_string(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    let inner_src = String::from_utf8_lossy(node_bytes(source, inner)).into_owned();
    let replacement = if cond.start_byte() == if_kw.end_byte() {
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

fn has_negation_op(source: &SourceFile, node: Node<'_>) -> bool {
    let mut cur = node.walk();
    node.children(&mut cur).any(|c| {
        let t = node_bytes(source, c);
        t == b"!" || t == b"not"
    })
}

fn negated_operand<'a>(source: &SourceFile, cond: Node<'a>) -> Option<Node<'a>> {
    if cond.kind() != "unary" || !has_negation_op(source, cond) {
        return None;
    }
    let operand = cond.child_by_field_name("operand")?;
    // `!!x` / `not not x` — RuboCop does not suggest `unless !x`.
    if operand.kind() == "unary" && has_negation_op(source, operand) {
        return None;
    }
    Some(operand)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegatedIf, "cops/style/negated_if");
}
