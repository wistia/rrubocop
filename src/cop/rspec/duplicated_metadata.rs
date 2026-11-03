use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct DuplicatedMetadata;
impl Cop for DuplicatedMetadata {
    fn name(&self) -> &'static str {
        "RSpec/DuplicatedMetadata"
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
        if !metadata_call(source, node) {
            return;
        }
        let args = call_args(node);
        let mut seen = std::collections::HashSet::new();
        for (index, arg) in args.iter().enumerate().skip(1) {
            if !matches!(arg.kind(), "simple_symbol" | "symbol" | "delimited_symbol") {
                continue;
            }
            let Some(value) = symbol(source, *arg) else {
                continue;
            };
            if seen.insert(value) {
                continue;
            }
            let diagnostic = report(
                self,
                source,
                arg.start_byte(),
                arg.end_byte(),
                "Avoid duplicated metadata.".into(),
                diagnostics,
            );
            if preserving_replace(
                source,
                node,
                &mut corrections,
                args[index - 1].end_byte(),
                arg.end_byte(),
                "",
                self.name(),
            ) {
                diagnostics[diagnostic].corrected = true;
            }
        }
    }
}
