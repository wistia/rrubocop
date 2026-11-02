use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct DescribeMethod;
impl Cop for DescribeMethod {
    fn name(&self) -> &'static str {
        "RSpec/DescribeMethod"
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
        if safe_send(source, node) {
            return;
        }
        if bare_rspec_call(source, node) != Some(b"describe")
            || !top_groups(source, node).contains(&node)
        {
            return;
        }
        let args = call_args(node);
        let Some(second) = args.get(1).copied() else {
            return;
        };
        if !matches!(second.kind(), "string" | "chained_string")
            || literal_starts_with(source, second, &[b'.', b'#'])
        {
            return;
        }
        report(self,source,second.start_byte(),second.end_byte(),"The second argument to describe should be the method being tested. '#instance' or '.class'.".into(),diagnostics);
    }
}
