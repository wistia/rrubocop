use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct SpecFilePathSuffix;
impl Cop for SpecFilePathSuffix {
    fn name(&self) -> &'static str {
        "RSpec/SpecFilePathSuffix"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["program"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if source.path.to_string_lossy().ends_with("_spec.rb") {
            return;
        }
        for _group in top_groups(source, node).into_iter().filter(|group| {
            crate::cop::rspec::helpers::bare_rspec_call(source, *group)
                .is_some_and(is_example_group)
        }) {
            report(
                self,
                source,
                0,
                1,
                "Spec path should end with `_spec.rb`.".into(),
                diagnostics,
            );
        }
    }
}
