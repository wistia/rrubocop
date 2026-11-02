use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ReceiveNever;
impl Cop for ReceiveNever {
    fn name(&self) -> &'static str {
        "RSpec/ReceiveNever"
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
        if name(source, node) != Some(b"never") || !contains_bare_method(source, node, b"receive") {
            return;
        }
        let target = selector(node);
        let index = report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            "Use `not_to receive` instead of `never`.".into(),
            diagnostics,
        );
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        let Some(parent) = node.parent().and_then(|arg| {
            if arg.kind() == "argument_list" {
                arg.parent()
            } else {
                Some(arg)
            }
        }) else {
            return;
        };
        if !runner_including_safe(source, parent)
            || !call_args(node).is_empty()
            || node.child_by_field_name("arguments").is_some()
        {
            return;
        }
        let runner = selector(parent);
        if !preserves_comments(source, parent, receiver.end_byte(), target.end_byte(), "") {
            return;
        }
        if preserving_replace(
            source,
            parent,
            &mut corrections,
            runner.start_byte(),
            runner.end_byte(),
            "not_to",
            self.name(),
        ) {
            preserving_replace(
                source,
                parent,
                &mut corrections,
                receiver.end_byte(),
                target.end_byte(),
                "",
                self.name(),
            );
            diagnostics[index].corrected = true;
        }
    }
}
