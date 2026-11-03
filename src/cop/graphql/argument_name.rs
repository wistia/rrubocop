//! GraphQL/ArgumentName — argument names must be snake_case.

use tree_sitter::Node;

use super::helpers::{CALL_KINDS, DEPT_INCLUDE, argument_name, is_argument_call, is_snake_case};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArgumentName;

impl Cop for ArgumentName {
    fn name(&self) -> &'static str {
        "GraphQL/ArgumentName"
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
        let Some(name) = argument_name(source, node) else {
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
            "Use snake_case for argument names".into(),
        ));
    }
}
