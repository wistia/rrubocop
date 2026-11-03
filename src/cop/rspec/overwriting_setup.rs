use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct OverwritingSetup;
impl Cop for OverwritingSetup {
    fn name(&self) -> &'static str {
        "RSpec/OverwritingSetup"
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if numbered_block(source, node)
            || !bare_rspec_call(source, node).is_some_and(is_example_group)
        {
            return;
        }
        let Some(body) = call_block(node).and_then(|block| block.child_by_field_name("body"))
        else {
            return;
        };
        let mut seen = std::collections::HashSet::new();
        for child in code_children(body) {
            if numbered_block(source, child)
                || call_block(child).is_none()
                || call_receiver(child).is_some()
                || safe_send(source, child)
                || !matches!(
                    name(source, child),
                    Some(b"let" | b"let!" | b"subject" | b"subject!")
                )
            {
                continue;
            }
            let args = call_args(child);
            let value = if args.is_empty() {
                "subject".to_string()
            } else if args.len() == 1
                && (symbol_node(args[0])
                    || matches!(args[0].kind(), "string" | "heredoc_beginning" | "character"))
            {
                let Some(value) = crate::cop::ported::static_literal(source, args[0]) else {
                    continue;
                };
                value
            } else {
                continue;
            };
            if !seen.insert(value.clone()) {
                report(
                    self,
                    source,
                    child.start_byte(),
                    child.end_byte(),
                    format!("`{value}` is already defined."),
                    diagnostics,
                );
            }
        }
    }
}
