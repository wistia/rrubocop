//! Metrics/ModuleLength — module body line count vs Max.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ModuleLength;

fn nested_skip_ranges(node: Node<'_>) -> Vec<(usize, usize)> {
    let Some(body) = node.child_by_field_name("body") else {
        return Vec::new();
    };
    let mut cur = body.walk();
    body.named_children(&mut cur)
        .filter(|c| matches!(c.kind(), "class" | "module"))
        .map(|c| (c.start_position().row + 1, c.end_position().row + 1))
        .collect()
}

fn body_line_count(source: &SourceFile, node: Node<'_>, count_comments: bool) -> usize {
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    if end_line <= start_line + 1 {
        return 0;
    }
    let mut next = start_line + 1;
    let mut count = 0;
    for (start, end) in nested_skip_ranges(node) {
        count += super::line_counts::count(source, next, start.min(end_line), count_comments);
        next = next.max(end.saturating_add(1));
    }
    count + super::line_counts::count(source, next, end_line, count_comments)
}

/// RuboCop `namespace_module?` — body is a nested `class`/`module` only (`sclass` excluded).
fn is_namespace_module(node: Node<'_>) -> bool {
    let Some(body) = node.child_by_field_name("body") else {
        return false;
    };
    let mut cur = body.walk();
    let kids: Vec<_> = body.named_children(&mut cur).collect();
    kids.len() == 1 && matches!(kids[0].kind(), "class" | "module")
}

impl Cop for ModuleLength {
    fn name(&self) -> &'static str {
        "Metrics/ModuleLength"
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["module"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if is_namespace_module(node) {
            return;
        }
        let max = config.get_usize("Max", 100);
        let length = body_line_count(source, node, config.get_bool("CountComments", false));
        if length <= max {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            format!("Module has too many lines. [{length}/{max}]"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(ModuleLength, "cops/metrics/module_length");

    #[test]
    fn singleton_class_body_counts_toward_length() {
        let cfg = CopConfig {
            options: HashMap::from([(
                "Max".into(),
                serde_yml::Value::Number(serde_yml::Number::from(5usize)),
            )]),
            ..CopConfig::default()
        };
        let diags = run_cop_full_with_config(
            &ModuleLength,
            b"module M\n  class << self\n    def a; 1; end\n    def b; 2; end\n    def c; 3; end\n    def d; 4; end\n    def e; 5; end\n    def f; 6; end\n  end\nend\n",
            cfg,
        );
        assert_eq!(
            diags.len(),
            1,
            "class << self must not be treated as namespace: {diags:?}"
        );
    }
}
