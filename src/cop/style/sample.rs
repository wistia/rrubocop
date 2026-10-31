//! Style/Sample — prefer sample over shuffle.first etc.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, call_receiver, node_text};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Sample;

impl Cop for Sample {
    fn name(&self) -> &'static str {
        "Style/Sample"
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
        let Some(root) = detect(source, node) else {
            return;
        };
        report(self, source, node, root, diagnostics, &mut corrections);
    }
}

fn detect<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    if !is_shuffle_index(source, node) {
        return None;
    }
    let recv = call_receiver(node)?;
    call_receiver(recv)
}

fn report(
    cop: &Sample,
    source: &SourceFile,
    node: Node<'_>,
    root: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let root_src = node_text(source, root);
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Use `sample` instead of `shuffle` followed by indexing.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: node.start_byte(),
            end: node.end_byte(),
            replacement: format!("{root_src}.sample"),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn is_shuffle_index(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(method) = call_method_name(source, node) else {
        return false;
    };
    if !matches!(method, b"first" | b"last" | b"at" | b"[]" | b"slice") {
        return false;
    }
    let Some(recv) = call_receiver(node) else {
        return false;
    };
    recv.kind() == "call" && call_method_name(source, recv) == Some(b"shuffle")
}
