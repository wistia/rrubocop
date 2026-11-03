//! GraphQL/FieldName — field names must be snake_case.

use tree_sitter::Node;

use super::helpers::{CALL_KINDS, DEPT_INCLUDE, field_name, is_field_call, is_snake_case};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FieldName;

impl Cop for FieldName {
    fn name(&self) -> &'static str {
        "GraphQL/FieldName"
    }

    fn default_include(&self) -> &'static [&'static str] {
        DEPT_INCLUDE
    }

    fn safe_autocorrect(&self) -> bool {
        false
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        CALL_KINDS
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        super::helpers::FIELD_METHODS
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !is_field_call(source, node) {
            return;
        }
        let Some(name) = field_name(source, node) else {
            return;
        };
        if is_snake_case(&name) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "Use snake_case for field names".into(),
        ));
    }
}
