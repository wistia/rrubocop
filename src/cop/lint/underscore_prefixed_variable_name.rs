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
        for scope in file_model.scopes.iter() {
            for (name, entry) in &scope.entries {
                if !name.starts_with('_') || name.as_ref() == "_" {
                    continue;
                }
                if entry.reads.is_empty() {
                    continue;
                }
                let (line, col) = file_model.line_col(entry.intro_byte);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    col,
                    "Do not use prefix `_` for a variable that is used.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnderscorePrefixedVariableName,
        "cops/lint/underscore_prefixed_variable_name"
    );
}
