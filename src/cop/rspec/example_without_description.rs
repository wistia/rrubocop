use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ExampleWithoutDescription;
impl Cop for ExampleWithoutDescription {
    fn name(&self) -> &'static str {
        "RSpec/ExampleWithoutDescription"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        let Some(method) = name(source, node) else {
            return;
        };
        let Some(block) = call_block(node) else {
            return;
        };
        if call_receiver(node).is_some() || !crate::cop::rspec::helpers::is_example(method) {
            return;
        }
        let args = call_args(node);
        if args.len() == 1 && args[0].kind() == "string" && symbol(source, args[0]) == Some(b"") {
            report(
                self,
                source,
                args[0].start_byte(),
                args[0].end_byte(),
                "Omit the argument when you want to have auto-generated description.".into(),
                diagnostics,
            );
            return;
        }
        let multiline = node.start_position().row != block.end_position().row;
        let style = config.get_str("EnforcedStyle", "always_allow");
        if !args.is_empty()
            || !(style == "disallow" || style == "single_line_only" && multiline)
            || method == b"specify" && multiline
        {
            return;
        }
        report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            "Add a description.".into(),
            diagnostics,
        );
    }
}
