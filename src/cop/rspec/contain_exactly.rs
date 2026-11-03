use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::node_text;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ContainExactly;
impl Cop for ContainExactly {
    fn name(&self) -> &'static str {
        "RSpec/ContainExactly"
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
        if !bare(source, node, b"contain_exactly") {
            return;
        }
        let args = call_args(node);
        if args.is_empty() || args.iter().any(|arg| arg.kind() != "splat_argument") {
            return;
        }
        let values: Vec<_> = args
            .iter()
            .filter_map(|arg| arg.named_child(0))
            .map(|arg| node_text(source, arg))
            .collect();
        if values.len() != args.len() {
            return;
        }
        let index = report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            "Prefer `match_array` when matching array values.".into(),
            diagnostics,
        );
        if preserving_replace(
            source,
            node,
            &mut corrections,
            node.start_byte(),
            header_end(node),
            format!("match_array({})", values.join(" + ")),
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
