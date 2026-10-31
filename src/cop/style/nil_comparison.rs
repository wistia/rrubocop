//! Style/NilComparison — prefer nil? over == nil / === nil.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NilComparison;

impl Cop for NilComparison {
    fn name(&self) -> &'static str {
        "Style/NilComparison"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["binary"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(kids) = detect(source, node) else {
            return;
        };
        report(self, source, node, &kids, diagnostics, &mut corrections);
    }
}

/// RuboCop `nil_comparison?` is `(send _ {:== :===} nil)` — not `!=` or `nil == x`.
fn detect<'a>(source: &'a SourceFile, node: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut cur = node.walk();
    let kids: Vec<_> = node.children(&mut cur).collect();
    if kids.len() < 3 {
        return None;
    }
    let op = node_bytes(source, kids[1]);
    if (op == b"==" || op == b"===") && node_bytes(source, kids[2]) == b"nil" {
        Some(kids)
    } else {
        None
    }
}

fn report(
    cop: &NilComparison,
    source: &SourceFile,
    node: Node<'_>,
    kids: &[Node<'_>],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Prefer the use of the `nil?` predicate.".to_string(),
    );
    if let Some(corr) = corrections {
        corr.push(Correction {
            start: kids[0].end_byte(),
            end: node.end_byte(),
            replacement: ".nil?".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NilComparison, "cops/style/nil_comparison");
}
