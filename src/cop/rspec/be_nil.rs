use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct BeNil;
impl Cop for BeNil {
    fn name(&self) -> &'static str {
        "RSpec/BeNil"
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
        let args = call_args(node);
        let style = config.get_str("EnforcedStyle", "be_nil");
        let (message, replacement) = if style == "be_nil"
            && name(source, node) == Some(b"be")
            && args.len() == 1
            && args[0].kind() == "nil"
        {
            ("Prefer `be_nil` over `be(nil)`.", "be_nil")
        } else if style == "be" && name(source, node) == Some(b"be_nil") && args.is_empty() {
            ("Prefer `be(nil)` over `be_nil`.", "be(nil)")
        } else {
            return;
        };
        let index = report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            message.into(),
            diagnostics,
        );
        if preserving_replace(
            source,
            node,
            &mut corrections,
            node.start_byte(),
            header_end(node),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
