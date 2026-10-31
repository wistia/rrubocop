//! Style/Proc — prefer `proc` over `Proc.new`.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, call_receiver, is_const_named};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Proc;

impl Cop for Proc {
    fn name(&self) -> &'static str {
        "Style/Proc"
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
        let Some(recv) = detect(source, node) else {
            return;
        };
        report(self, source, node, recv, diagnostics, &mut corrections);
    }
}

fn detect<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    if call_method_name(source, node) != Some(b"new") {
        return None;
    }
    let parent = node.parent()?;
    if parent.kind() != "block" {
        return None;
    }
    if parent.child_by_field_name("method")? != node {
        return None;
    }
    let recv = call_receiver(node)?;
    if !is_const_named(source, recv, b"Proc") {
        return None;
    }
    Some(recv)
}

fn report(
    cop: &Proc,
    source: &SourceFile,
    node: Node<'_>,
    recv: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(recv.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Use `proc` instead of `Proc.new`.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: recv.start_byte(),
            end: node
                .child_by_field_name("method")
                .unwrap_or(node)
                .end_byte(),
            replacement: "proc".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
