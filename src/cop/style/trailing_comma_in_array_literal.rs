//! Style/TrailingCommaInArrayLiteral.

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct TrailingCommaInArrayLiteral;

impl Cop for TrailingCommaInArrayLiteral {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInArrayLiteral"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["array"]
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

    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &TrailingCommaInArrayLiteral,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trailing_comma_in_array_literal/parity.json"
            )),
        );
    }

    #[test]
    fn consistent_comma_single_elem_bracket_same_line_ok() {
        let mut config = CopConfig::default();
        config.options.insert(
            "EnforcedStyleForMultiline".into(),
            serde_yml::Value::String("consistent_comma".into()),
        );
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &TrailingCommaInArrayLiteral,
            b"x = [{\n  a: 1,\n}]\n",
            config,
        );
    }
}
