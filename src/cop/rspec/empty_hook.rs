use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct EmptyHook;
impl Cop for EmptyHook {
    fn name(&self) -> &'static str {
        "RSpec/EmptyHook"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        if call_receiver(node).is_some()
            || !name(source, node).is_some_and(|name| hooks().contains(&name))
        {
            return;
        }
        let Some(block) = call_block(node) else {
            return;
        };
        if block
            .child_by_field_name("body")
            .is_some_and(|body| !code_children(body).is_empty())
        {
            return;
        }
        let index = report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            "Empty hook detected.".into(),
            diagnostics,
        );
        let mut start = node.start_byte();
        while start > 0 && source.as_bytes()[start - 1].is_ascii_whitespace() {
            start -= 1;
        }
        if preserving_replace(
            source,
            node,
            &mut corrections,
            start,
            node.end_byte(),
            "",
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
