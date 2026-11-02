use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct DeleteSuffix;
impl Cop for DeleteSuffix {
    fn name(&self) -> &'static str {
        "Performance/DeleteSuffix"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"gsub", b"gsub!", b"sub", b"sub!"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        super::anchored_substitution::check(
            self,
            true,
            source,
            node,
            config,
            diagnostics,
            corrections,
        );
    }
}
