//! Layout/MultilineMethodCallBraceLayout.

use tree_sitter::Node;

use crate::cop::layout::brace_layout;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMethodCallBraceLayout;

fn last_argument_heredoc(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|n| !matches!(n.kind(), "comment" | "heredoc_body" | "heredoc_end"))
        .last()
        .is_some_and(|n| n.kind() == "heredoc_beginning")
}

impl Cop for MultilineMethodCallBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodCallBraceLayout"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["argument_list"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if last_argument_heredoc(node) {
            return;
        }
        brace_layout::check_braces(
            self,
            source,
            node,
            b'(',
            config.get_str("EnforcedStyle", "symmetrical"),
            diagnostics,
            &mut corrections,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MultilineMethodCallBraceLayout,
        "cops/layout/multiline_method_call_brace_layout"
    );
}
