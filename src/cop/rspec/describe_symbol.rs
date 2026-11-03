use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct DescribeSymbol;
impl Cop for DescribeSymbol {
    fn name(&self) -> &'static str {
        "RSpec/DescribeSymbol"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        if bare_rspec_call(source, node) != Some(b"describe") {
            return;
        }
        let Some(arg) = call_args(node).first().copied() else {
            return;
        };
        if !matches!(arg.kind(), "simple_symbol" | "symbol" | "delimited_symbol") {
            return;
        }
        report(
            self,
            source,
            arg.start_byte(),
            arg.end_byte(),
            "Avoid describing symbols.".into(),
            diagnostics,
        );
    }
}
