use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct MessageChain;
impl Cop for MessageChain {
    fn name(&self) -> &'static str {
        "RSpec/MessageChain"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) = name(source, node) else {
            return;
        };
        if ![b"receive_message_chain".as_slice(), b"stub_chain"].contains(&method) {
            return;
        }
        let target = selector(node);
        report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            format!(
                "Avoid stubbing using `{}`.",
                String::from_utf8_lossy(method)
            ),
            diagnostics,
        );
    }
}
