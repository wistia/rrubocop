//! Prefer an explicit join model for many-to-many associations.
use crate::cop::shared::{call_method_name, call_receiver, method_node};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct HasAndBelongsToMany;
impl Cop for HasAndBelongsToMany {
    fn name(&self) -> &'static str {
        "Rails/HasAndBelongsToMany"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"has_and_belongs_to_many"]
    }
    fn interested_identifier_names(&self) -> &'static [&'static [u8]] {
        &[b"has_and_belongs_to_many"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if node.kind() == "identifier" {
            if crate::cop::shared::node_bytes(source, node) != b"has_and_belongs_to_many"
                || crate::cop::ported::local_identifier(source, node)
                || node
                    .parent()
                    .is_some_and(|parent| parent.child_by_field_name("method") == Some(node))
            {
                return;
            }
        } else if call_method_name(source, node) != Some(b"has_and_belongs_to_many")
            || call_receiver(node).is_some()
        {
            return;
        }
        let selector = method_node(node).unwrap_or(node);
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Prefer `has_many :through` to `has_and_belongs_to_many`.".into(),
        );
        d.highlight_length = selector.end_byte() - selector.start_byte();
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HasAndBelongsToMany, "cops/rails/has_and_belongs_to_many");
}
