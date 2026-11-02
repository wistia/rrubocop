//! Style/TrailingCommaInHashLiteral.

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct TrailingCommaInHashLiteral;

impl Cop for TrailingCommaInHashLiteral {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInHashLiteral"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["hash"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        super::trailing_comma_literal::check_literal(
            self,
            source,
            node,
            config,
            diagnostics,
            corrections,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::assert_cop_no_offenses_full_with_config;

    crate::cop_fixture_tests!(
        TrailingCommaInHashLiteral,
        "cops/style/trailing_comma_in_hash_literal"
    );

    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &TrailingCommaInHashLiteral,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_hash_literal/parity.json"
            )),
        );
    }

    #[test]
    fn comma_before_trailing_comment_consistent() {
        let mut config = CopConfig::default();
        config.options.insert(
            "EnforcedStyleForMultiline".into(),
            serde_yml::Value::String("consistent_comma".into()),
        );
        assert_cop_no_offenses_full_with_config(
            &TrailingCommaInHashLiteral,
            b"{\n  a: 1,\n  b: 2, # comment\n}\n",
            config,
        );
    }
}
