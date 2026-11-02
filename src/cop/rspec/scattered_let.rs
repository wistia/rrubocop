use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::ordering::{self, Style};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ScatteredLet;
impl Cop for ScatteredLet {
    fn name(&self) -> &'static str {
        "RSpec/ScatteredLet"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
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
        corrections: Option<&mut Vec<Correction>>,
    ) {
        ordering::check(
            self,
            Style::ScatteredLet,
            source,
            node,
            diagnostics,
            corrections,
        );
    }
}
