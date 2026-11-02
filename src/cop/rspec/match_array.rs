use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{call_receiver, node_text};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct MatchArray;
impl Cop for MatchArray {
    fn name(&self) -> &'static str {
        "RSpec/MatchArray"
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
        if name(source, node) != Some(b"match_array") {
            return;
        }
        let args = call_args(node);
        let Some(array) = args.first().copied().filter(|arg| arg.kind() == "array") else {
            return;
        };
        if call_receiver(node).is_none() && code_children(array).is_empty() && args.len() == 1 {
            return;
        }
        let mut values = Vec::new();
        for arg in args {
            values.extend(
                code_children(arg)
                    .into_iter()
                    .map(|child| node_text(source, child)),
            );
        }
        let index = report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            "Prefer `contain_exactly` when matching an array literal.".into(),
            diagnostics,
        );
        if call_receiver(node).is_none()
            && preserving_replace(
                source,
                node,
                &mut corrections,
                node.start_byte(),
                header_end(node),
                format!("contain_exactly({})", values.join(", ")),
                self.name(),
            )
        {
            diagnostics[index].corrected = true;
        }
    }
}
