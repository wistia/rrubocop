//! Metrics/ClassLength — class body line count vs Max.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClassLength;

/// Lines strictly inside the class (between `class`/`end`), RuboCop-style.
fn body_line_count(source: &SourceFile, node: Node<'_>, count_comments: bool) -> usize {
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    if end_line <= start_line + 1 {
        return 0;
    }
    super::line_counts::count(source, start_line + 1, end_line, count_comments)
}

fn check_class(
    cop: &ClassLength,
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let max = config.get_usize("Max", 100);
    let length = body_line_count(source, node, config.get_bool("CountComments", false));
    if length <= max {
        return;
    }
    let (line, col) = source.offset_to_line_col(node.start_byte());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        col,
        format!("Class has too many lines. [{length}/{max}]"),
    ));
}

impl Cop for ClassLength {
    fn name(&self) -> &'static str {
        "Metrics/ClassLength"
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["class", "singleton_class"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if node.kind() == "singleton_class" {
            // RuboCop skips `class << self` nested inside another class.
            let mut p = node.parent();
            while let Some(n) = p {
                if n.kind() == "class" {
                    return;
                }
                p = n.parent();
            }
        }
        check_class(self, source, node, config, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassLength, "cops/metrics/class_length");
}
