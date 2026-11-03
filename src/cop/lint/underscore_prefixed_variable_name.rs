use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/UnderscorePrefixedVariableName — `_x` that is used.
pub struct UnderscorePrefixedVariableName;

impl Cop for UnderscorePrefixedVariableName {
    fn name(&self) -> &'static str {
        "Lint/UnderscorePrefixedVariableName"
    }

    fn redundant_disable_audit(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn needs_file_model(&self) -> bool {
        true
    }

    fn check_file_model(
        &self,
        source: &SourceFile,
        file_model: &crate::model::FileModel<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut implicit_blocks = std::collections::HashSet::new();
        let mut nodes = vec![file_model.tree.root_node()];
        while let Some(node) = nodes.pop() {
            if matches!(node.kind(), "block" | "do_block" | "lambda")
                && node.child_by_field_name("parameters").is_none()
            {
                implicit_blocks.insert(node.start_byte());
            }
            let mut cursor = node.walk();
            nodes.extend(node.named_children(&mut cursor));
        }
        for scope in file_model.scopes.iter() {
            for (name, entry) in &scope.entries {
                if !name.starts_with('_') || name.as_ref() == "_" {
                    continue;
                }
                if matches!(name.as_bytes(), [b'_', b'1'..=b'9'])
                    && implicit_blocks.contains(&scope.entered_at)
                    && entry.intro_kind == crate::model::IntroKind::Binding
                    && entry.writes.is_empty()
                {
                    continue;
                }
                if entry.reads.is_empty() {
                    continue;
                }
                let (line, col) = file_model.line_col(entry.intro_byte);
                let mut diagnostic = self.diagnostic(
                    source,
                    line,
                    col,
                    "Do not use prefix `_` for a variable that is used.".to_string(),
                );
                diagnostic.highlight_length = name.chars().count();
                diagnostics.push(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_numbered_parameter_roles() {
        crate::testutil::assert_original_spec_parity(
            &UnderscorePrefixedVariableName,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/underscore_prefixed_variable_name/numbered-parameters.json"
            )),
        );
    }
    crate::cop_fixture_tests!(
        UnderscorePrefixedVariableName,
        "cops/lint/underscore_prefixed_variable_name"
    );
}
