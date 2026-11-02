use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{argument_nodes, call_receiver};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct VerifiedDoubles;
impl Cop for VerifiedDoubles {
    fn name(&self) -> &'static str {
        "RSpec/VerifiedDoubles"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_receiver(node).is_some() || !matches!(name(source, node), Some(b"double" | b"spy"))
        {
            return;
        }
        let args = argument_nodes(node);
        if args.is_empty() && config.get_bool("IgnoreNameless", true) {
            return;
        }
        if args
            .first()
            .is_some_and(|a| matches!(a.kind(), "symbol" | "simple_symbol"))
            && config.get_bool("IgnoreSymbolicNames", false)
        {
            return;
        }
        report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            "Prefer using verifying doubles over normal doubles.".into(),
            diagnostics,
        );
    }
}
