use crate::cop::rspec::helpers::{
    RSPEC_INCLUDE, bare_rspec_call, call_block, is_example, is_group,
};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn inclusion(source: &SourceFile, node: Node<'_>) -> bool {
    !safe_send(source, node)
        && matches!(
            name(source, node),
            Some(
                b"include_examples"
                    | b"include_context"
                    | b"it_behaves_like"
                    | b"it_should_behave_like"
            )
        )
}

pub struct LetSetup;
impl Cop for LetSetup {
    fn name(&self) -> &'static str {
        "RSpec/LetSetup"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _ = config;
        if numbered_block(source, node)
            || call_block(node).is_none()
            || !(bare_rspec_call(source, node).is_some_and(is_group)
                || call_receiver(node).is_none() && inclusion(source, node))
        {
            return;
        }
        let mut lets = Vec::new();
        let mut pending = code_children(node);
        while let Some(child) = pending.pop() {
            let args = call_args(child);
            let helper = !safe_send(source, child)
                && call_receiver(child).is_none()
                && matches!(name(source, child), Some(b"let" | b"let!"))
                && !numbered_block(source, child)
                && (call_block(child).is_some()
                    || args.len() == 2 && args[1].kind() == "block_argument");
            if helper {
                lets.push(child);
                continue;
            }
            if !numbered_block(source, child)
                && call_block(child).is_some()
                && (bare_rspec_call(source, child).is_some_and(is_group)
                    || call_receiver(child).is_none()
                        && (inclusion(source, child)
                            || bare_rspec_call(source, child).is_some_and(is_example)))
            {
                continue;
            }
            pending.extend(code_children(child));
        }
        lets.sort_by_key(Node::start_byte);
        for helper in lets {
            if name(source, helper) != Some(b"let!") {
                continue;
            }
            let args = call_args(helper);
            let valid = if call_block(helper).is_some() {
                args.len() == 1
            } else {
                args.len() == 2 && args[1].kind() == "block_argument"
            };
            if !valid {
                continue;
            }
            let argument = args[0];
            if !(symbol_node(argument)
                || matches!(
                    argument.kind(),
                    "string" | "character" | "heredoc_beginning"
                ))
            {
                continue;
            }
            let Some(value) = crate::cop::ported::static_literal(source, argument) else {
                continue;
            };
            let mut pending = vec![node];
            let mut used = false;
            while let Some(child) = pending.pop() {
                if !safe_send(source, child)
                    && call_receiver(child).is_none()
                    && call_args(child).is_empty()
                    && name(source, child) == Some(value.as_bytes())
                {
                    used = true;
                    break;
                }
                if child.kind() == "hash_key_symbol"
                    && child.parent().is_some_and(|pair| {
                        pair.kind() == "pair" && pair.child_by_field_name("value").is_none()
                    })
                    && node_bytes(source, child)
                        .strip_suffix(b":")
                        .unwrap_or(node_bytes(source, child))
                        == value.as_bytes()
                    && !local_binding(source, child)
                {
                    used = true;
                    break;
                }
                pending.extend(code_children(child));
            }
            if !used {
                report(
                    self,
                    source,
                    helper.start_byte(),
                    header_end(helper),
                    "Do not use `let!` to setup objects not referenced in tests.".into(),
                    diagnostics,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LetSetup, "cops/rspec/let_setup");
}
