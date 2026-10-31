//! Security/Eval — Kernel.eval / eval calls.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Eval;

fn kernel_eval(source: &SourceFile, node: Node<'_>) -> bool {
    crate::cop::shared::call_method_name(source, node) == Some(b"eval")
        && node.child_by_field_name("receiver").is_none_or(|n| {
            matches!(
                crate::cop::shared::node_bytes(source, n),
                b"Kernel" | b"::Kernel"
            )
        })
}

impl Cop for Eval {
    fn name(&self) -> &'static str {
        "Security/Eval"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !kernel_eval(source, node) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "The use of `eval` is a serious security risk.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Eval, "cops/security/eval");
}
