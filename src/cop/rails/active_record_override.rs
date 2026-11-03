//! Prefer callbacks to overriding persistence methods while forwarding all arguments.
use crate::cop::shared::{for_each_descendant, named_kids, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ActiveRecordOverride;
impl Cop for ActiveRecordOverride {
    fn name(&self) -> &'static str {
        "Rails/ActiveRecordOverride"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["method"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if node.kind() != "method" {
            return;
        }
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        let name = node_text(source, name);
        if !matches!(name.as_str(), "create" | "destroy" | "save" | "update") {
            return;
        }
        let mut parent = node.parent();
        let mut model = false;
        while let Some(ancestor) = parent {
            if ancestor.kind() == "class" {
                let superclass = ancestor
                    .child_by_field_name("superclass")
                    .and_then(|n| named_kids(n).first().copied());
                model = superclass.is_some_and(|n| {
                    matches!(
                        node_text(source, n).as_str(),
                        "ApplicationRecord" | "ActiveModel::Base" | "ActiveRecord::Base"
                    )
                });
                break;
            }
            parent = ancestor.parent();
        }
        if !model {
            return;
        }
        let mut forwarding = false;
        for_each_descendant(node, |n| {
            forwarding |= n.kind() == "super"
                && !n.parent().is_some_and(|parent| {
                    parent.kind() == "call" && parent.child_by_field_name("method") == Some(n)
                })
        });
        if !forwarding {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let message = format!(
            "Use `before_{name}`, `around_{name}`, or `after_{name}` callbacks instead of overriding the Active Record method `{name}`."
        );
        let mut d = self.diagnostic(source, line, col, message);
        d.highlight_length =
            super::blank::highlight_length(source, node.start_byte(), node.end_byte());
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActiveRecordOverride, "cops/rails/active_record_override");
}
