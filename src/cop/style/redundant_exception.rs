//! Style/RedundantException — raise RuntimeError, msg -> raise msg.

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, is_const_named};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantException;

impl Cop for RedundantException {
    fn name(&self) -> &'static str {
        "Style/RedundantException"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(first) = detect(source, node) else {
            return;
        };
        report(self, source, node, first, diagnostics, &mut corrections);
    }
}

fn detect<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let method = call_method_name(source, node)?;
    if method != b"raise" && method != b"fail" {
        return None;
    }
    let args = argument_nodes(node);
    if args.len() != 2 {
        return None;
    }
    if !is_const_named(source, args[0], b"RuntimeError") {
        return None;
    }
    Some(args[0])
}

fn report(
    cop: &RedundantException,
    source: &SourceFile,
    node: Node<'_>,
    first: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Redundant `RuntimeError` argument can be removed.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        let mut end = first.end_byte();
        let bytes = source.as_bytes();
        while end < bytes.len() && matches!(bytes[end], b' ' | b'\t' | b',') {
            end += 1;
        }
        corr.push(Correction {
            start: first.start_byte(),
            end,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
