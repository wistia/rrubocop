use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct SubjectDeclaration;
impl Cop for SubjectDeclaration {
    fn name(&self) -> &'static str {
        "RSpec/SubjectDeclaration"
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
        if call_receiver(node).is_some()
            || ![b"let".as_slice(), b"let!", b"subject", b"subject!"].contains(&method)
        {
            return;
        }
        let Some(first) = call_args(node).first().copied() else {
            return;
        };
        if !matches!(symbol(source, first), Some(b"subject" | b"subject!")) {
            return;
        }
        let message = if [b"let".as_slice(), b"let!"].contains(&method) {
            "Use subject explicitly rather than using let"
        } else {
            "Ambiguous declaration of subject"
        };
        report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            message.into(),
            diagnostics,
        );
    }
}
