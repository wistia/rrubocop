use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, node_text};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/ParenthesesAsGroupedExpression — `foo (1)` space before paren.
pub struct ParenthesesAsGroupedExpression;

fn spaced_paren_arg<'a>(source: &SourceFile, node: Node<'a>) -> Option<(usize, usize, Node<'a>)> {
    let args = argument_nodes(node);
    if args.len() != 1 || args[0].kind() != "parenthesized_statements" {
        return None;
    }
    let method = node.child_by_field_name("method")?;
    let between_start = method.end_byte();
    let between_end = args[0].start_byte();
    let between = &source.as_bytes()[between_start..between_end];
    if !between.iter().any(|&b| b == b' ' || b == b'\t') {
        return None;
    }
    Some((between_start, between_end, args[0]))
}

impl Cop for ParenthesesAsGroupedExpression {
    fn name(&self) -> &'static str {
        "Lint/ParenthesesAsGroupedExpression"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let Some((space_start, space_end, arg)) = spaced_paren_arg(source, node) else {
            return;
        };
        let text = node_text(source, arg);
        let (line, col) = source.offset_to_line_col(arg.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            format!("`{text}` interpreted as grouped expression."),
        );
        if let Some(corr) = corrections.as_mut() {
            // RuboCop removes the space so `foo (1)` → `foo(1)`.
            corr.push(Correction {
                start: space_start,
                end: space_end,
                replacement: String::new(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
