//! Style/RedundantFreeze — freeze on immutable.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, call_receiver};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantFreeze;

impl Cop for RedundantFreeze {
    fn name(&self) -> &'static str {
        "Style/RedundantFreeze"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"freeze"]
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
    if call_method_name(source, node) != Some(b"freeze") {
        return None;
    }
    let recv = call_receiver(node)?;
    if !matches!(
        recv.kind(),
        "integer" | "float" | "symbol" | "true" | "false" | "nil" | "complex" | "rational"
    ) {
        return None;
    }
    Some(recv)
}

fn report(
    cop: &RedundantFreeze,
    source: &SourceFile,
    node: Node<'_>,
    recv: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Do not freeze immutable objects, as freezing them has no effect.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: recv.end_byte(),
            end: node
                .child_by_field_name("method")
                .unwrap_or(node)
                .end_byte(),
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
