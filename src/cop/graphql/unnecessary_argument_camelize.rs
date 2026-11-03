//! GraphQL/UnnecessaryArgumentCamelize — camelize unused when name has no underscore.

use tree_sitter::Node;

use super::helpers::{CALL_KINDS, DEPT_INCLUDE, argument_name, has_kwarg, is_argument_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct UnnecessaryArgumentCamelize;

impl Cop for UnnecessaryArgumentCamelize {
    fn name(&self) -> &'static str {
        "GraphQL/UnnecessaryArgumentCamelize"
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
        if name.split('_').count() >= 2 {
            return;
        }
        if !has_kwarg(source, node, "camelize") {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "Unnecessary argument camelize".into(),
        ));
    }
}
