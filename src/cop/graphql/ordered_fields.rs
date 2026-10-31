//! GraphQL/OrderedFields — fields alphabetical within consecutive groups.

use tree_sitter::Node;

use super::helpers::{
    DEPT_INCLUDE, class_body_stmts, consecutive_lines, correct_order, field_name, is_field_call,
    module_body_stmts, nested_class,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct OrderedFields;

impl Cop for OrderedFields {
    fn name(&self) -> &'static str {
        "GraphQL/OrderedFields"
    }

    fn default_include(&self) -> &'static [&'static str] {
        DEPT_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["class"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if node.kind() == "class" && nested_class(node) {
            return;
        }
        let groups = config.get_bool("Groups", true);
        let order = super::helpers::config_string_list(config, "Order");
        let order_ref = (!order.is_empty()).then_some(order.as_slice());
        for win in body_fields(source, node).windows(2) {
            check_pair(self, source, win[0], win[1], groups, order_ref, diagnostics);
        }
    }
}

fn body_fields<'a>(source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    let body = if node.kind() == "module" {
        module_body_stmts(node)
    } else {
        class_body_stmts(node)
    };
    body.into_iter()
        .filter(|n| is_field_call(source, *n))
        .collect()
}

fn check_pair(
    cop: &OrderedFields,
    source: &SourceFile,
    prev: Node<'_>,
    curr: Node<'_>,
    groups: bool,
    order_ref: Option<&[String]>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if groups && !adjacent_fields(prev, curr) {
        return;
    }
    let (Some(pn), Some(cn)) = (field_name(source, prev), field_name(source, curr)) else {
        return;
    };
    if correct_order(&pn, &cn, order_ref) {
        return;
    }
    let (line, col) = source.offset_to_line_col(curr.start_byte());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        col,
        format!(
            "Fields should be sorted in an alphabetical order within their section. Field `{cn}` should appear before `{pn}`."
        ),
    ));
}

fn adjacent_fields(prev: Node<'_>, curr: Node<'_>) -> bool {
    if consecutive_lines(prev, curr) {
        return true;
    }
    prev.child_by_field_name("block")
        .map(|b| b.end_position().row)
        .unwrap_or(prev.end_position().row)
        + 1
        == curr.start_position().row
}
