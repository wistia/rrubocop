use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ContextMethod;
impl Cop for ContextMethod {
    fn name(&self) -> &'static str {
        "RSpec/ContextMethod"
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
        if bare_rspec_call(source, node) != Some(b"context") || call_block(node).is_none() {
            return;
        }
        let Some(arg) = call_args(node).first().copied() else {
            return;
        };
        if !matches!(arg.kind(), "string" | "chained_string") {
            return;
        }
        if !literal_starts_with(source, arg, &[b'.', b'#']) {
            return;
        }
        let index = report(
            self,
            source,
            arg.start_byte(),
            arg.end_byte(),
            "Use `describe` for testing methods.".into(),
            diagnostics,
        );
        let target = selector(node);
        if preserving_replace(
            source,
            node,
            &mut corrections,
            target.start_byte(),
            target.end_byte(),
            "describe",
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
