//! Style/InfiniteLoop — prefer Kernel#loop for while/until true/false.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InfiniteLoop;

impl Cop for InfiniteLoop {
    fn name(&self) -> &'static str {
        "Style/InfiniteLoop"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["while", "until", "while_modifier", "until_modifier"]
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
        if !is_infinite(source, node, cond) {
            return;
        }
        report(self, source, node, cond, diagnostics, &mut corrections);
    }
}

fn is_infinite(source: &SourceFile, node: Node<'_>, cond: Node<'_>) -> bool {
    let text = node_bytes(source, cond);
    matches!(
        (node.kind(), text),
        ("while" | "while_modifier", b"true") | ("until" | "until_modifier", b"false")
    )
}

fn report(
    cop: &InfiniteLoop,
    source: &SourceFile,
    node: Node<'_>,
    cond: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Use `Kernel#loop` for infinite loops.".to_string(),
    );
    if matches!(node.kind(), "while" | "until")
        && let Some(corr) = corrections.as_mut()
    {
        corr.push(Correction {
            start: node.start_byte(),
            end: after_condition_or_do(node, cond),
            replacement: "loop do".into(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn after_condition_or_do(node: Node<'_>, cond: Node<'_>) -> usize {
    let mut cur = node.walk();
    for ch in node.children(&mut cur) {
        if !ch.is_named() && ch.kind() == "do" && ch.start_byte() >= cond.end_byte() {
            return ch.end_byte();
        }
    }
    cond.end_byte()
}
