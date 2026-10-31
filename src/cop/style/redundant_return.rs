//! Style/RedundantReturn — avoid unnecessary return.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantReturn;

impl Cop for RedundantReturn {
    fn name(&self) -> &'static str {
        "Style/RedundantReturn"
    }

    fn redundant_disable_audit(&self) -> bool {
        false
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["return"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !is_trailing_return(node) {
            return;
        }
        if config.get_bool("AllowMultipleReturnValues", false) && returns_multiple(node) {
            return;
        }
        report(self, source, node, diagnostics, &mut corrections);
    }
}

fn returns_multiple(node: Node<'_>) -> bool {
    let mut cur = node.walk();
    let named: Vec<_> = node.named_children(&mut cur).collect();
    if named.len() > 1 {
        return true;
    }
    // `return a, b` → one `argument_list` child with multiple values.
    named.first().is_some_and(|n| {
        if n.kind() != "argument_list" {
            return false;
        }
        let mut c2 = n.walk();
        n.named_children(&mut c2).count() > 1
    })
}

fn report(
    cop: &RedundantReturn,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Redundant `return` detected.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        push_remove_return(cop, node, corr);
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn push_remove_return(cop: &RedundantReturn, node: Node<'_>, corr: &mut Vec<Correction>) {
    if returns_multiple(node) {
        // RuboCop: `return a, b` → `[a, b]` (bare `a, b` is invalid Ruby).
        wrap_multi_return_as_array(cop, node, corr);
        return;
    }
    let mut cur = node.walk();
    let named: Vec<_> = node.named_children(&mut cur).collect();
    let end = if named.is_empty() {
        node.end_byte()
    } else {
        named[0].start_byte()
    };
    corr.push(Correction {
        start: node.start_byte(),
        end,
        replacement: String::new(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
}

fn wrap_multi_return_as_array(cop: &RedundantReturn, node: Node<'_>, corr: &mut Vec<Correction>) {
    let mut cur = node.walk();
    let named: Vec<_> = node.named_children(&mut cur).collect();
    let Some(first) = named.first() else {
        return;
    };
    let last = named.last().unwrap_or(first);
    // Remove `return` (+ following space) and wrap values in `[…]`.
    corr.push(Correction {
        start: node.start_byte(),
        end: first.start_byte(),
        replacement: "[".into(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    corr.push(Correction {
        start: last.end_byte(),
        end: last.end_byte(),
        replacement: "]".into(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
}

fn is_trailing_return(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let Some(gp) = parent.parent() else {
        return false;
    };
    let container = if parent.kind() == "body_statement" {
        gp
    } else {
        parent
    };
    if !matches!(container.kind(), "method" | "singleton_method") {
        // `return` inside a block exits the enclosing method — never redundant.
        return false;
    }
    let Some(body) = return_body(parent, container) else {
        return false;
    };
    let mut cur = body.walk();
    body.named_children(&mut cur).last().map(|n| n.id()) == Some(node.id())
}

fn return_body<'a>(parent: Node<'a>, container: Node<'a>) -> Option<Node<'a>> {
    if parent.kind() == "body_statement" {
        Some(parent)
    } else {
        container.child_by_field_name("body")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::assert_cop_no_offenses_full_with_config;

    crate::cop_fixture_tests!(RedundantReturn, "cops/style/redundant_return");

    #[test]
    fn allow_multiple_return_values_no_offense() {
        let mut config = CopConfig::default();
        config.options.insert(
            "AllowMultipleReturnValues".into(),
            serde_yml::Value::Bool(true),
        );
        assert_cop_no_offenses_full_with_config(
            &RedundantReturn,
            b"def foo\n  return a, b\nend\n",
            config,
        );
    }
}
