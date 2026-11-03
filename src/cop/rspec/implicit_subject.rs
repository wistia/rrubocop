use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block, is_example};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ImplicitSubject;
impl Cop for ImplicitSubject {
    fn name(&self) -> &'static str {
        "RSpec/ImplicitSubject"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) || call_receiver(node).is_some() {
            return;
        }
        let Some(method) = name(source, node) else {
            return;
        };
        let style = config.get_str("EnforcedStyle", "single_line_only");
        let implicit = matches!(method, b"is_expected" | b"should" | b"should_not");
        let mut example = None;
        let mut ancestor = node.parent();
        while let Some(parent) = ancestor {
            if bare_rspec_call(source, parent).is_some_and(is_example)
                && call_block(parent).is_some()
            {
                example = Some(parent);
                break;
            }
            ancestor = parent.parent();
        }
        let invalid = if style == "require_implicit" {
            let args = call_args(node);
            method == b"expect"
                && args.len() == 1
                && bare(source, args[0], b"subject")
                && call_args(args[0]).is_empty()
        } else {
            implicit
                && !example.is_some_and(|example| name(source, example) == Some(b"its"))
                && match style {
                    "disallow" => true,
                    "single_line_only" => !example.is_some_and(|example| {
                        example.start_position().row == example.end_position().row
                    }),
                    "single_statement_only" => example
                        .and_then(call_block)
                        .and_then(|block| block.child_by_field_name("body"))
                        .is_some_and(|body| code_children(body).len() > 1),
                    _ => false,
                }
        };
        if !invalid {
            return;
        }
        let message = if style == "require_implicit" {
            "Don't use explicit subject."
        } else {
            "Don't use implicit subject."
        };
        let index = report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            message.into(),
            diagnostics,
        );
        let target = selector(node);
        let (start, end, replacement) = match method {
            b"expect" => (node.start_byte(), header_end(node), "is_expected"),
            b"is_expected" => (target.start_byte(), target.end_byte(), "expect(subject)"),
            b"should" => (target.start_byte(), target.end_byte(), "expect(subject).to"),
            _ => (
                target.start_byte(),
                target.end_byte(),
                "expect(subject).not_to",
            ),
        };
        if preserving_replace(
            source,
            node,
            &mut corrections,
            start,
            end,
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
