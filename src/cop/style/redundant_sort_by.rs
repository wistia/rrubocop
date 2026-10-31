//! Style/RedundantSortBy — sort_by identity → sort.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantSortBy;

impl Cop for RedundantSortBy {
    fn name(&self) -> &'static str {
        "Style/RedundantSortBy"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"sort_by"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"sort_by") || !is_identity_sort_by(source, node)
        {
            return;
        }
        report(self, source, node, diagnostics, &mut corrections);
    }
}

fn report(
    cop: &RedundantSortBy,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let meth = node.child_by_field_name("method").unwrap_or(node);
    let (line, col) = source.offset_to_line_col(meth.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Use `sort` instead of `sort_by { |x| x }` / `sort_by(&:itself)`.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: meth.start_byte(),
            end: node.end_byte(),
            replacement: "sort".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn is_identity_sort_by(source: &SourceFile, node: Node<'_>) -> bool {
    let mut cur = node.walk();
    node.children(&mut cur)
        .any(|child| identity_child(source, child))
}

fn identity_child(source: &SourceFile, child: Node<'_>) -> bool {
    match child.kind() {
        "block_argument" => node_bytes(source, child) == b"&:itself",
        "block" | "do_block" => identity_block_text(node_bytes(source, child)),
        _ => false,
    }
}

fn identity_block_text(raw: &[u8]) -> bool {
    match split_block_parts(&String::from_utf8_lossy(raw)) {
        Some((param, body)) => !param.is_empty() && param == body && !param.contains(','),
        None => false,
    }
}

fn split_block_parts(text: &str) -> Option<(&str, String)> {
    if !text.contains('|') {
        return None;
    }
    let parts: Vec<_> = text.split('|').collect();
    if parts.len() < 3 {
        return None;
    }
    Some((
        parts[1].trim(),
        parts[2]
            .trim()
            .trim_end_matches("end")
            .trim()
            .trim_matches(|c| c == '{' || c == '}')
            .to_string(),
    ))
}
