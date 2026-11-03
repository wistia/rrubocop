use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
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
        if safe_send(source, node) {
            return;
        }
        if !runner(source, node) {
            return;
        }
        let args = call_args(node);
        let Some(matcher) = args.first().copied() else {
            return;
        };
        if args.len() != 1
            || !bare(source, matcher, b"be")
            || !call_args(matcher).is_empty()
            || call_block(matcher).is_some()
        {
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

#[cfg(test)]
mod tests {
    #[test]
    fn original_block_binding_and_production_regressions() {
        crate::testutil::assert_original_spec_parity(
            &super::Be,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/production-matchers/rspec-be.json"
            )),
        );
    }
}
