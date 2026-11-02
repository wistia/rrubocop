use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ImplicitExpect;
impl Cop for ImplicitExpect {
    fn name(&self) -> &'static str {
        "RSpec/ImplicitExpect"
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
        let style = config.get_str("EnforcedStyle", "is_expected");
        let method = name(source, node);
        let (start, end, current, replacement) =
            if matches!(method, Some(b"should" | b"should_not")) && call_receiver(node).is_none() {
                if style == "should" {
                    return;
                }
                let current = String::from_utf8_lossy(method.unwrap()).into_owned();
                let replacement = if method == Some(b"should") {
                    "is_expected.to"
                } else {
                    "is_expected.to_not"
                };
                let target = selector(node);
                (
                    target.start_byte(),
                    target.end_byte(),
                    current,
                    replacement.to_string(),
                )
            } else if runner(source, node)
                && call_receiver(node).is_some_and(|recv| {
                    bare(source, recv, b"is_expected") && call_args(recv).is_empty()
                })
            {
                if style == "is_expected" {
                    return;
                }
                let end = selector(node).end_byte();
                let current = String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                    .into_owned();
                let replacement = if method == Some(b"to") {
                    "should"
                } else {
                    "should_not"
                };
                (node.start_byte(), end, current, replacement.to_string())
            } else {
                return;
            };
        let index = report(
            self,
            source,
            start,
            end,
            format!("Prefer `{replacement}` over `{current}`."),
            diagnostics,
        );
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
