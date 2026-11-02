use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct AroundBlock;
impl Cop for AroundBlock {
    fn name(&self) -> &'static str {
        "RSpec/AroundBlock"
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !bare(source, node, b"around") || safe_send(source, node) {
            return;
        }
        let args = call_args(node);
        if args.len() > 1 || args.iter().any(|arg| !symbol_node(*arg)) {
            return;
        }
        let Some(block) = call_block(node) else {
            return;
        };
        let parameters = block.child_by_field_name("parameters");
        let numbered = numbered_block(source, node);
        let proxy = parameters.and_then(|parameters| code_children(parameters).first().copied());
        if proxy.is_none() && !numbered {
            report(
                self,
                source,
                node.start_byte(),
                node.end_byte(),
                "Test object should be passed to around block.".into(),
                diagnostics,
            );
            return;
        }
        let variable = proxy
            .map(|proxy| node_bytes(source, proxy.child_by_field_name("name").unwrap_or(proxy)))
            .unwrap_or(b"_1");
        let mut pending = code_children(block);
        let local = |node: Node<'_>| {
            node.kind() == "identifier"
                && node_bytes(source, node) == variable
                && (crate::cop::ported::local_identifier(source, node)
                    || numbered
                        && variable == b"_1"
                        && node.parent().is_none_or(|parent| {
                            parent.child_by_field_name("method") != Some(node)
                        }))
        };
        while let Some(child) = pending.pop() {
            if !safe_send(source, child) {
                let arguments = call_args(child);
                if call_receiver(child).is_some_and(local)
                    && matches!(name(source, child), Some(b"run" | b"call"))
                    && arguments.is_empty()
                {
                    return;
                }
                if name(source, child).is_some() && arguments.iter().any(|arg| local(*arg)) {
                    return;
                }
                if child.kind() == "yield"
                    && code_children(child)
                        .iter()
                        .filter(|child| child.kind() == "argument_list")
                        .flat_map(|arguments| code_children(*arguments))
                        .any(local)
                {
                    return;
                }
            }
            if child.kind() == "block_argument"
                && code_children(child).iter().any(|arg| local(*arg))
            {
                return;
            }
            pending.extend(code_children(child));
        }
        let target = proxy
            .or_else(|| block.child_by_field_name("body"))
            .unwrap_or(block);
        report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            format!(
                "You should call `{}.call` or `{}.run`.",
                String::from_utf8_lossy(variable),
                String::from_utf8_lossy(variable)
            ),
            diagnostics,
        );
    }
}
