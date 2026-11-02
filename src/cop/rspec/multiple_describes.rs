use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct MultipleDescribes;
impl Cop for MultipleDescribes {
    fn name(&self) -> &'static str {
        "RSpec/MultipleDescribes"
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
        if !bare_rspec_call(source, node).is_some_and(is_example_group) {
            return;
        }
        let groups: Vec<_> = top_groups(source, node)
            .into_iter()
            .filter(|group| bare_rspec_call(source, *group).is_some_and(is_example_group))
            .collect();
        if groups.len() < 2 || groups.first() != Some(&node) {
            return;
        }
        report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            "Do not use multiple top-level example groups - try to nest them.".into(),
            diagnostics,
        );
    }
}
