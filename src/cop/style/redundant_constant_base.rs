//! Style/RedundantConstantBase — avoid ::Foo at top level.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantConstantBase;

impl Cop for RedundantConstantBase {
    fn name(&self) -> &'static str {
        "Style/RedundantConstantBase"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["scope_resolution"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if node.child_by_field_name("scope").is_some() || !starts_with_colon2(source, node) {
            return;
        }
        // RuboCop only flags leading `::` when the constant is already at top-level
        // scope; nested `::Foo` inside classes/modules is kept. Approximate: parent
        // must be the program root (no class/module enclosure in between).
        if !is_top_level(node) {
            return;
        }
        report(self, source, node, diagnostics, &mut corrections);
    }
}

fn is_top_level(node: Node<'_>) -> bool {
    let mut p = node.parent();
    while let Some(n) = p {
        match n.kind() {
            "program" => return true,
            "class" | "module" | "singleton_class" | "method" | "singleton_method" | "block"
            | "do_block" | "lambda" => return false,
            _ => p = n.parent(),
        }
    }
    false
}

fn starts_with_colon2(source: &SourceFile, node: Node<'_>) -> bool {
    let bytes = source.as_bytes();
    let s = node.start_byte();
    s + 1 < bytes.len() && bytes[s] == b':' && bytes[s + 1] == b':'
}

fn report(
    cop: &RedundantConstantBase,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Remove redundant leading `::`.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: node.start_byte(),
            end: node.start_byte() + 2,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
