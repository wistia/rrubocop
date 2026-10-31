//! Style/OperatorMethodCall — avoid `.+` style calls.

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, call_receiver, node_text};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct OperatorMethodCall;

impl Cop for OperatorMethodCall {
    fn name(&self) -> &'static str {
        "Style/OperatorMethodCall"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((method, recv, op)) = detect(source, node) else {
            return;
        };
        report(
            self,
            source,
            node,
            method,
            recv,
            op,
            diagnostics,
            &mut corrections,
        );
    }
}

fn detect<'a>(source: &'a SourceFile, node: Node<'a>) -> Option<(&'a [u8], Node<'a>, Node<'a>)> {
    let method = call_method_name(source, node)?;
    let recv = call_receiver(node)?;
    if !is_op(method) || node.child_by_field_name("arguments").is_none() {
        return None;
    }
    // RuboCop: only `.` method calls (not `&.`); never `[]` / `[]=`.
    let op = node.child_by_field_name("operator")?;
    if node_text(source, op) != "." {
        return None;
    }
    Some((method, recv, op))
}

fn is_op(method: &[u8]) -> bool {
    matches!(
        method,
        b"+" | b"-" | b"*" | b"/" | b"%" | b"|" | b"&" | b"^" | b"<<" | b">>"
    )
}

fn report(
    cop: &OperatorMethodCall,
    source: &SourceFile,
    node: Node<'_>,
    method: &[u8],
    recv: Node<'_>,
    op: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(op.start_byte());
    let mut diag = cop.diagnostic(source, line, col, "Redundant dot detected.".to_string());
    if let Some(corr) = corrections.as_mut() {
        push_op_form(cop, source, node, method, recv, corr, &mut diag);
    }
    diagnostics.push(diag);
}

fn push_op_form(
    cop: &OperatorMethodCall,
    source: &SourceFile,
    node: Node<'_>,
    method: &[u8],
    recv: Node<'_>,
    corr: &mut Vec<Correction>,
    diag: &mut Diagnostic,
) {
    let args = argument_nodes(node);
    if args.len() != 1 {
        return;
    }
    let op = String::from_utf8_lossy(method);
    let left = node_text(source, recv);
    let right = node_text(source, args[0]);
    corr.push(Correction {
        start: node.start_byte(),
        end: node.end_byte(),
        replacement: format!("{left} {op} {right}"),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    diag.corrected = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OperatorMethodCall, "cops/style/operator_method_call");
}
