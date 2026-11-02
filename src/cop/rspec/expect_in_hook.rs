use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ExpectInHook;
impl Cop for ExpectInHook {
    fn name(&self) -> &'static str {
        "RSpec/ExpectInHook"
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
        if call_receiver(node).is_some()
            || !name(source, node).is_some_and(|method| expectations().contains(&method))
        {
            return;
        }
        let mut ancestor = node.parent();
        while let Some(parent) = ancestor {
            if call_receiver(parent).is_none()
                && name(source, parent).is_some_and(|method| hooks().contains(&method))
                && call_block(parent).is_some_and(|block| block.start_byte() <= node.start_byte())
            {
                let target = selector(node);
                let expectation = String::from_utf8_lossy(name(source, node).unwrap());
                let hook = String::from_utf8_lossy(name(source, parent).unwrap());
                report(
                    self,
                    source,
                    target.start_byte(),
                    target.end_byte(),
                    format!("Do not use `{expectation}` in `{hook}` hook"),
                    diagnostics,
                );
            }
            ancestor = parent.parent();
        }
    }
}
