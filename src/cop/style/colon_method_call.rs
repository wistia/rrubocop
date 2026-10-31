//! Style/ColonMethodCall
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ColonMethodCall;

impl Cop for ColonMethodCall {
    fn name(&self) -> &'static str {
        "Style/ColonMethodCall"
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
        _c: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(op) = colon_method_op(source, node) else {
            return;
        };
        let (l, c) = source.offset_to_line_col(op.start_byte());
        let mut diag = self.diagnostic(source, l, c, "Do not use `::` for method calls.".into());
        if let Some(corr) = corrections.as_mut() {
            corr.push(Correction {
                start: op.start_byte(),
                end: op.end_byte(),
                replacement: ".".into(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

fn colon_method_op<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let op = find_colon_op(node)?;
    if !method_is_lowercase(source, node) || is_java_recv(source, node) {
        return None;
    }
    Some(op)
}

fn find_colon_op(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|ch| !ch.is_named() && ch.kind() == "::")
}

fn method_is_lowercase(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(method) = node.child_by_field_name("method") else {
        return false;
    };
    let mb = node_bytes(source, method);
    !mb.is_empty() && !mb[0].is_ascii_uppercase()
}

fn is_java_recv(source: &SourceFile, node: Node<'_>) -> bool {
    call_receiver(node)
        .is_some_and(|recv| recv.kind() == "constant" && node_bytes(source, recv) == b"Java")
}
