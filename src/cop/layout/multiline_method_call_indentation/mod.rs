//! Layout/MultilineMethodCallIndentation.

mod chain;
mod dot_col;

use tree_sitter::{Node, Tree};

use crate::cop::layout::multiline_operation_indentation;
use crate::cop::layout::report;
use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct MultilineMethodCallIndentation;

fn expected_col(
    source: &SourceFile,
    node: Node<'_>,
    recv: Node<'_>,
    style: &str,
    width: usize,
) -> usize {
    match style {
        "indented_relative_to_receiver" => shared::line_indent(source, recv.start_byte()) + width,
        "indented" => indented_chain_col(source, node, width),
        "aligned" | _ => aligned_expected(source, node, width),
    }
}

fn indented_chain_col(source: &SourceFile, node: Node<'_>, width: usize) -> usize {
    let root = chain::chain_root(source, node);
    shared::line_indent(source, root.start_byte())
        + width
        + if prefix_keyword(
            source
                .line_text(shared::node_line(source, root))
                .unwrap_or("")
                .trim_start(),
        ) {
            width
        } else {
            0
        }
}

fn prefix_keyword(line: &str) -> bool {
    (line.starts_with("return ") && !line.contains(" if ") && !line.contains(" unless "))
        || line.starts_with("if ")
        || line.starts_with("unless ")
}

fn aligned_expected(source: &SourceFile, node: Node<'_>, width: usize) -> usize {
    if let Some(col) = dot_col::dot_aligned_above(source, node) {
        return col;
    }
    if let Some(col) = dot_col::first_same_line_dot_col(source, node) {
        return col;
    }
    // RuboCop falls back to `indentation(chain) + width` when there is no
    // same-line first dot (`allow(...)\n  .to\n  .and_call_original`).
    multiline_operation_indentation::aligned_method_call_col(
        source,
        node,
        chain::chain_root(source, node),
        width,
    )
}

fn inside_paren_arg_list(node: Node<'_>) -> bool {
    let mut p = node.parent();
    while let Some(n) = p {
        if n.kind() == "parenthesized_statements" {
            return true;
        }
        if n.kind() == "argument_list" {
            return n.child(0).is_some_and(|c| !c.is_named() && c.kind() == "(");
        }
        if matches!(
            n.kind(),
            "program" | "method" | "singleton_method" | "class" | "module"
        ) {
            break;
        }
        p = n.parent();
    }
    false
}

fn check_call(
    cop: &dyn Cop,
    source: &SourceFile,
    n: Node<'_>,
    style: &str,
    width: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some((method, expected, actual)) = call_indent_mismatch(source, n, style, width) else {
        return;
    };
    report::fix_indent(
        cop,
        source,
        method.start_byte(),
        format!(
            "Align method call receivers and their chained calls consistently (expected column {expected})."
        ),
        diagnostics,
        corrections,
        actual,
        expected,
    );
}

fn call_indent_mismatch<'a>(
    source: &SourceFile,
    n: Node<'a>,
    style: &str,
    width: usize,
) -> Option<(Node<'a>, usize, usize)> {
    let (recv, method) = leading_dot_parts(source, n)?;
    let expected = expected_col(source, n, recv, style, width);
    let actual = shared::line_indent(source, method.start_byte());
    (actual != expected).then_some((method, expected, actual))
}

fn leading_dot_parts<'a>(source: &SourceFile, n: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    if n.kind() != "call" || !chain::has_dot(source, n) || inside_paren_arg_list(n) {
        return None;
    }
    let recv = n.child_by_field_name("receiver")?;
    let method = n.child_by_field_name("method")?;
    if shared::node_bytes(source, method) == b"[]" {
        return None;
    }
    let (recv_end_line, _) = source.offset_to_line_col(recv.end_byte().saturating_sub(1));
    if recv_end_line == shared::node_line(source, method) {
        return None;
    }
    Some((recv, method))
}

impl Cop for MultilineMethodCallIndentation {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodCallIndentation"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        tree: &Tree,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = code_map;
        let width = config.get_usize("IndentationWidth", 2);
        let style = config.get_str("EnforcedStyle", "aligned");
        shared::for_each_descendant(tree.root_node(), |n| {
            check_call(self, source, n, style, width, diagnostics, &mut corrections);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    crate::cop_fixture_tests!(
        MultilineMethodCallIndentation,
        "cops/layout/multiline_method_call_indentation"
    );

    #[test]
    fn indented_rspec_chains() {
        let mut config = CopConfig::default();
        config.options.insert(
            "EnforcedStyle".into(),
            serde_yml::Value::String("indented".into()),
        );
        config.options.insert(
            "IndentationWidth".into(),
            serde_yml::Value::Number(2.into()),
        );
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &MultilineMethodCallIndentation,
            b"expect { post(:create) }\n  .to change(A, :count).by(1)\n  .and(change(B, :count).by(1))\n",
            config,
        );
    }

    #[test]
    fn no_offense_indented_chain_fixture() {
        crate::testutil::assert_cop_no_offenses_full(
            &MultilineMethodCallIndentation,
            include_bytes!(
                "../../../../tests/fixtures/cops/layout/multiline_method_call_indentation/no_offense_indented_chain.rb"
            ),
        );
    }

    #[test]
    fn no_offense_dot_above_fixture() {
        crate::testutil::assert_cop_no_offenses_full(
            &MultilineMethodCallIndentation,
            include_bytes!(
                "../../../../tests/fixtures/cops/layout/multiline_method_call_indentation/no_offense_dot_above.rb"
            ),
        );
    }

    #[test]
    fn no_offense_comment_between_fixture() {
        crate::testutil::assert_cop_no_offenses_full(
            &MultilineMethodCallIndentation,
            include_bytes!(
                "../../../../tests/fixtures/cops/layout/multiline_method_call_indentation/no_offense_comment_between.rb"
            ),
        );
    }

    #[test]
    fn no_offense_multiline_args_chain_fixture() {
        crate::testutil::assert_cop_no_offenses_full(
            &MultilineMethodCallIndentation,
            include_bytes!(
                "../../../../tests/fixtures/cops/layout/multiline_method_call_indentation/no_offense_multiline_args_chain.rb"
            ),
        );
    }
}
