//! GraphQL/LegacyDsl — ban GraphQL::*Type.define blocks.

use tree_sitter::Node;

use super::helpers::{CALL_KINDS, DEPT_INCLUDE};
use crate::cop::shared::{call_method_name, call_receiver, is_const_named, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LegacyDsl;

impl Cop for LegacyDsl {
    fn name(&self) -> &'static str {
        "GraphQL/LegacyDsl"
    }

    fn default_include(&self) -> &'static [&'static str] {
        DEPT_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        CALL_KINDS
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"define"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"define") {
            return;
        }
        let Some(recv) = call_receiver(node) else {
            return;
        };
        if !is_graphql_type_const(source, recv) {
            return;
        }
        if node.child_by_field_name("block").is_none() {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "Avoid using legacy based type-based definitions. Use class-based definitions instead."
                .into(),
        ));
    }
}

fn is_graphql_type_const(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() != "scope_resolution" {
        return false;
    }
    let Some(scope) = node.child_by_field_name("scope") else {
        return false;
    };
    is_const_named(source, scope, b"GraphQL") || node_text(source, scope).starts_with("GraphQL")
}
