use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct RemoveConst;
impl Cop for RemoveConst {
    fn name(&self) -> &'static str {
        "RSpec/RemoveConst"
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
        if safe_send(source, node) {
            return;
        }
        if !matches!(name(source, node), Some(b"send" | b"__send__")) {
            return;
        }
        let args = call_args(node);
        if args.len() != 2
            || !matches!(
                args[0].kind(),
                "symbol" | "simple_symbol" | "delimited_symbol"
            )
            || symbol(source, args[0]) != Some(b"remove_const")
        {
            return;
        }
        report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            "Do not use remove_const in specs. Consider using e.g. `stub_const`.".into(),
            diagnostics,
        );
    }
}
