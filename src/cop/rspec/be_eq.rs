use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct BeEq;
impl Cop for BeEq {
    fn name(&self) -> &'static str {
        "RSpec/BeEq"
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
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = config;
        if safe_send(source, node) || !bare(source, node, b"eq") {
            return;
        }
        let args = call_args(node);
        if args.len() != 1 || !matches!(args[0].kind(), "true" | "false" | "nil") {
            return;
        }
        let matcher = node;
        let message = "Prefer `be` over `eq`.".into();
        let replacement = "be";
        let target = selector(matcher);
        let index = report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            message,
            diagnostics,
        );
        if preserving_replace(
            source,
            matcher,
            &mut corrections,
            target.start_byte(),
            target.end_byte(),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
