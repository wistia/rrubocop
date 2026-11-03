//! Style/MissingRespondToMissing.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MissingRespondToMissing;

fn semantic_parent(node: Node<'_>) -> Option<Node<'_>> {
    let parent = node.parent()?;
    if node.kind() == "block" || node.kind() == "do_block" {
        if parent.kind() == "call" {
            return semantic_parent(parent);
        }
    }
    if parent.kind() == "argument_list" {
        return semantic_parent(parent);
    }
    if matches!(
        parent.kind(),
        "program" | "body_statement" | "block_body" | "then" | "else"
    ) {
        let mut cursor = parent.walk();
        if parent
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "comment")
            .count()
            == 1
        {
            return semantic_parent(parent);
        }
    }
    Some(parent)
}

fn implements_respond_to_missing(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(scope) = semantic_parent(node).and_then(semantic_parent) else {
        return false;
    };
    let mut found = false;
    crate::cop::shared::for_each_descendant(scope, |candidate| {
        if candidate.id() == scope.id() || candidate.kind() != node.kind() {
            return;
        }
        found |= candidate
            .child_by_field_name("name")
            .is_some_and(|name| node_bytes(source, name) == b"respond_to_missing?");
        if let Some(object) = candidate.child_by_field_name("object") {
            found |= call_method_name(source, object) == Some(b"respond_to_missing?")
                || (object.kind() == "identifier"
                    && node_bytes(source, object) == b"respond_to_missing?");
        }
    });
    found
}

impl Cop for MissingRespondToMissing {
    fn name(&self) -> &'static str {
        "Style/MissingRespondToMissing"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["method", "singleton_method"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !node
            .child_by_field_name("name")
            .is_some_and(|name| node_bytes(source, name) == b"method_missing")
            || implements_respond_to_missing(source, node)
        {
            return;
        }
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "When using `method_missing`, define `respond_to_missing?`.".into(),
        );
        diagnostic.highlight_length = String::from_utf8_lossy(node_bytes(source, node))
            .chars()
            .count();
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standalone_method_reports_instead_of_crashing() {
        let diagnostics =
            crate::testutil::run_cop_full(&MissingRespondToMissing, b"def method_missing; end\n");
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &MissingRespondToMissing,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/missing_respond_to_missing/parity.json"
            )),
        );
    }
}
