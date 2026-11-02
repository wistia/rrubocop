use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct SkipBlockInsideExample;
impl Cop for SkipBlockInsideExample {
    fn name(&self) -> &'static str {
        "RSpec/SkipBlockInsideExample"
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
        if name(source, node) != Some(b"skip") || call_block(node).is_none() {
            return;
        }
        let mut ancestor = node.parent();
        while let Some(parent) = ancestor {
            if call_receiver(parent).is_none()
                && name(source, parent).is_some_and(crate::cop::rspec::helpers::is_example)
                && call_block(parent).is_some()
            {
                report(
                    self,
                    source,
                    node.start_byte(),
                    node.end_byte(),
                    "Don't pass a block to `skip` inside examples.".into(),
                    diagnostics,
                );
                return;
            }
            ancestor = parent.parent();
        }
    }
}
