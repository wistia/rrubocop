use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct EmptyOutput;
impl Cop for EmptyOutput {
    fn name(&self) -> &'static str {
        "RSpec/EmptyOutput"
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
        if !runner(source, node) {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        if !bare(source, receiver, b"expect") || call_block(receiver).is_none() {
            return;
        }
        let args = call_args(node);
        if args.len() != 1 {
            return;
        }
        let Some(output) = call_receiver(args[0]) else {
            return;
        };
        let values = call_args(output);
        if !bare(source, output, b"output")
            || values.len() != 1
            || values[0].kind() != "string"
            || symbol(source, values[0]) != Some(b"")
        {
            return;
        }
        let replacement = if name(source, node) == Some(b"to") {
            "not_to"
        } else {
            "to"
        };
        let index = report(
            self,
            source,
            output.start_byte(),
            header_end(output),
            format!("Use `{replacement}` instead of matching on an empty output."),
            diagnostics,
        );
        if !preserves_comments(
            source,
            output,
            output.start_byte(),
            header_end(output),
            "output",
        ) {
            return;
        }
        let target = selector(node);
        if preserving_replace(
            source,
            node,
            &mut corrections,
            target.start_byte(),
            target.end_byte(),
            replacement,
            self.name(),
        ) {
            preserving_replace(
                source,
                node,
                &mut corrections,
                output.start_byte(),
                header_end(output),
                "output",
                self.name(),
            );
            diagnostics[index].corrected = true;
        }
    }
}
