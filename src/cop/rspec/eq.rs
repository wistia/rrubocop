use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct Eq;
impl Cop for Eq {
    fn name(&self) -> &'static str {
        "RSpec/Eq"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !runner(source, node) {
            return;
        }
        let args = call_args(node);
        if args.len() != 1 {
            return;
        }
        let matcher = args[0];
        if matcher.kind() != "binary" {
            return;
        }
        let Some(left) = matcher.child_by_field_name("left") else {
            return;
        };
        let Some(operator) = matcher.child_by_field_name("operator") else {
            return;
        };
        if node_bytes(source, operator) != b"=="
            || !bare(source, left, b"be")
            || !call_args(left).is_empty()
        {
            return;
        }
        let index = report(
            self,
            source,
            matcher.start_byte(),
            operator.end_byte(),
            "Use `eq` instead of `be ==` to compare objects.".into(),
            diagnostics,
        );
        if preserving_replace(
            source,
            node,
            &mut corrections,
            matcher.start_byte(),
            operator.end_byte(),
            "eq",
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
