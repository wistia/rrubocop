use tree_sitter::Node;

use crate::cop::shared::node_text;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/LiteralInInterpolation — literal-only interpolation.
pub struct LiteralInInterpolation;

fn is_basic_literal(node: Node<'_>) -> bool {
    match node.kind() {
        "integer" | "float" | "simple_symbol" | "true" | "false" | "nil" | "string" | "regex" => {
            true
        }
        "array" | "hash" | "pair" | "range" => {
            let mut cur = node.walk();
            node.named_children(&mut cur).all(is_basic_literal)
        }
        _ => false,
    }
}

fn literal_replacement(source: &SourceFile, lit: Node<'_>) -> Option<String> {
    match lit.kind() {
        "integer" | "float" | "true" | "false" => Some(node_text(source, lit)),
        "nil" => Some(String::new()),
        "simple_symbol" => Some(node_text(source, lit).trim_start_matches(':').to_string()),
        _ => None,
    }
}

fn sole_literal(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    let named: Vec<_> = node.named_children(&mut cur).collect();
    if named.len() != 1 || !is_basic_literal(named[0]) || named[0].kind() == "string" {
        return None;
    }
    Some(named[0])
}

impl Cop for LiteralInInterpolation {
    fn name(&self) -> &'static str {
        "Lint/LiteralInInterpolation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["interpolation"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(lit) = sole_literal(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Literal interpolation detected.".to_string(),
        );
        if let Some(repl) = literal_replacement(source, lit)
            && let Some(corr) = corrections.as_mut()
        {
            corr.push(Correction {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement: repl,
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LiteralInInterpolation, "cops/lint/literal_in_interpolation");
}
