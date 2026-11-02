use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ItBehavesLike;
impl Cop for ItBehavesLike {
    fn name(&self) -> &'static str {
        "RSpec/ItBehavesLike"
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
        if safe_send(source, node) {
            return;
        }
        let style = config.get_str("EnforcedStyle", "it_behaves_like");
        let wrong = if style == "it_behaves_like" {
            b"it_should_behave_like".as_slice()
        } else {
            b"it_behaves_like".as_slice()
        };
        if name(source, node) != Some(wrong) {
            return;
        }
        let index = report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            format!(
                "Prefer `{style}` over `{}` when including examples in a nested context.",
                String::from_utf8_lossy(wrong)
            ),
            diagnostics,
        );
        let target = selector(node);
        if preserving_replace(
            source,
            node,
            &mut corrections,
            target.start_byte(),
            target.end_byte(),
            style,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
