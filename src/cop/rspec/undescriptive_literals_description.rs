use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct UndescriptiveLiteralsDescription;
impl Cop for UndescriptiveLiteralsDescription {
    fn name(&self) -> &'static str {
        "RSpec/UndescriptiveLiteralsDescription"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) = bare_rspec_call(source, node) else {
            return;
        };
        if call_block(node).is_none()
            || !(is_example_group(method) || crate::cop::rspec::helpers::is_example(method))
        {
            return;
        }
        let args = call_args(node);
        if args.len() != 1 {
            return;
        }
        let first = args[0];
        if !matches!(first.kind(), "integer" | "regex" | "subshell") {
            return;
        }
        report(
            self,
            source,
            first.start_byte(),
            first.end_byte(),
            "Description should be descriptive.".into(),
            diagnostics,
        );
    }
}
