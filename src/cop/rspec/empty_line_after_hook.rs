use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::{next_code_sibling, separate};
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct EmptyLineAfterHook;
impl Cop for EmptyLineAfterHook {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterHook"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        let Some(method) = name(source, node) else {
            return;
        };
        if call_receiver(node).is_some() || !hooks().contains(&method) || call_block(node).is_none()
        {
            return;
        }
        if config.get_bool("AllowConsecutiveOneLiners", true)
            && node.start_position().row == node.end_position().row
            && next_code_sibling(node).is_some_and(|next| {
                call_receiver(next).is_none()
                    && name(source, next).is_some_and(|method| hooks().contains(&method))
                    && call_block(next).is_some()
                    && next.start_position().row == next.end_position().row
            })
        {
            return;
        }
        separate(
            self,
            source,
            node,
            format!(
                "Add an empty line after `{}`.",
                String::from_utf8_lossy(method)
            ),
            diagnostics,
            &mut corrections,
        );
    }
}
