//! GraphQL/ArgumentDescription — arguments need a description.

use tree_sitter::Node;

use super::helpers::{CALL_KINDS, DEPT_INCLUDE, has_description, is_argument_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArgumentDescription;

impl Cop for ArgumentDescription {
    fn name(&self) -> &'static str {
        "GraphQL/ArgumentDescription"
    }

    fn redundant_disable_audit(&self) -> bool {
        false
    }

    fn default_include(&self) -> &'static [&'static str] {
        DEPT_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        CALL_KINDS
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        super::helpers::ARGUMENT_METHODS
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !is_argument_call(source, node) {
            return;
        }
        if has_description(source, node) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(source, line, col, "Missing argument description".into()));
    }
}
