use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct StartWith;
impl Cop for StartWith {
    fn name(&self) -> &'static str {
        "Performance/StartWith"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"match", b"match?", b"=~"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        super::anchored_match::check(self, false, source, node, config, diagnostics, corrections);
    }
}
