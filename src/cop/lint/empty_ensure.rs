use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/EmptyEnsure — empty `ensure` body.
pub struct EmptyEnsure;

fn ensure_keyword(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|ch| !ch.is_named() && ch.kind() == "ensure")
}

fn has_ensure_body(node: Node<'_>) -> bool {
    let mut cur = node.walk();
    node.named_children(&mut cur).any(|c| c.kind() != "ensure")
}

fn remove_ensure(cop_name: &'static str, kw: Node<'_>) -> Correction {
    Correction {
        start: kw.start_byte(),
        end: kw.end_byte(),
        replacement: String::new(),
        cop_name,
        cop_index: 0,
        owner: None,
    }
}

impl Cop for EmptyEnsure {
    fn name(&self) -> &'static str {
        "Lint/EmptyEnsure"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["ensure"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if has_ensure_body(node) {
            return;
        }
        let kw = ensure_keyword(node).unwrap_or(node);
        let (line, col) = source.offset_to_line_col(kw.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Empty `ensure` block detected.".to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            corr.push(remove_ensure(self.name(), kw));
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
