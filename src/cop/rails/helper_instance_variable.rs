use crate::cop::shared::{named_kids, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct HelperInstanceVariable;
fn form_builder(source: &SourceFile, node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if ancestor.kind() == "class"
            && ancestor
                .child_by_field_name("superclass")
                .and_then(|s| named_kids(s).into_iter().find(|n| n.kind() != "comment"))
                .and_then(|s| crate::cop::ported::constant_name(source, s))
                .as_deref()
                == Some("ActionView::Helpers::FormBuilder")
        {
            return true;
        }
        parent = ancestor.parent();
    }
    false
}
impl Cop for HelperInstanceVariable {
    fn name(&self) -> &'static str {
        "Rails/HelperInstanceVariable"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["app/helpers/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["instance_variable"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if node.kind() != "instance_variable" || form_builder(source, node) {
            return;
        }
        if node.parent().is_some_and(|p| {
            p.kind() == "operator_assignment"
                && p.child_by_field_name("left") == Some(node)
                && p.child_by_field_name("operator")
                    .is_some_and(|op| node_text(source, op) == "||=")
        }) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Do not use instance variables in helpers.".into(),
        );
        d.highlight_length =
            super::blank::highlight_length(source, node.start_byte(), node.end_byte());
        if let Some(s) = config.severity {
            d.severity = s;
        }
        diagnostics.push(d);
    }
}
