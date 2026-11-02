use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct EmptyMetadata;
impl Cop for EmptyMetadata {
    fn name(&self) -> &'static str {
        "RSpec/EmptyMetadata"
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
        if !metadata_call(source, node) {
            return;
        }
        let args = call_args(node);
        if args.len() < 2 {
            return;
        }
        let hash = *args.last().unwrap();
        if hash.kind() != "hash" || hash.named_child_count() != 0 {
            return;
        }
        let index = report(
            self,
            source,
            hash.start_byte(),
            hash.end_byte(),
            "Avoid empty metadata hash.".into(),
            diagnostics,
        );
        let previous = args[args.len() - 2];
        if preserving_replace(
            source,
            node,
            &mut corrections,
            previous.end_byte(),
            hash.end_byte(),
            "",
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
