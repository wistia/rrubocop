//! Style/Strip — prefer strip over lstrip.rstrip.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Strip;

impl Cop for Strip {
    fn name(&self) -> &'static str {
        "Style/Strip"
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
    let method = call_method_name(source, node)?;
    let recv = call_receiver(node)?;
    if recv.kind() != "call" {
        return None;
    }
    let inner = call_method_name(source, recv)?;
    let ok = matches!(
        (method, inner),
        (b"rstrip", b"lstrip") | (b"lstrip", b"rstrip")
    );
    if !ok {
        return None;
    }
    call_receiver(recv)
}

fn report(
    cop: &Strip,
    source: &SourceFile,
    node: Node<'_>,
    root: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let root_src = String::from_utf8_lossy(node_bytes(source, root));
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Use `strip` instead of `lstrip.rstrip`.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: node.start_byte(),
            end: node.end_byte(),
            replacement: format!("{root_src}.strip"),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
