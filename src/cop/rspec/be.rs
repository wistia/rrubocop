use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct Be;
impl Cop for Be {
    fn name(&self) -> &'static str {
        "RSpec/Be"
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
        if !runner(source, node) {
            return;
        }
        let args = call_args(node);
        let Some(matcher) = args.first().copied() else {
            return;
        };
        if args.len() != 1 || !bare(source, matcher, b"be") || !call_args(matcher).is_empty() {
            return;
        }
        let target = selector(matcher);
        report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            "Don't use `be` without an argument.".into(),
            diagnostics,
        );
    }
}
