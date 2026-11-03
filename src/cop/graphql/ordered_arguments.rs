//! GraphQL/OrderedArguments — arguments alphabetical within consecutive groups.

use tree_sitter::Node;

use super::helpers::{
    DEPT_INCLUDE, argument_name, consecutive_lines, correct_order, enclosing_class,
    is_argument_call, nested_class,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct OrderedArguments;

impl Cop for OrderedArguments {
    fn name(&self) -> &'static str {
        "GraphQL/OrderedArguments"
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
        if nested_class(node) {
            return;
        }
        let order = super::helpers::config_string_list(config, "Order");
        let order_ref = (!order.is_empty()).then_some(order.as_slice());
        let mut args = Vec::new();
        collect_args(source, node, node, &mut args);
        for win in args.windows(2) {
            report_pair(self, source, win[0], win[1], order_ref, diagnostics);
        }
    }
}

fn report_pair(
    cop: &OrderedArguments,
    source: &SourceFile,
    prev: Node<'_>,
    curr: Node<'_>,
    order_ref: Option<&[String]>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !consecutive_lines(prev, curr) {
        return;
    }
    let (Some(pn), Some(cn)) = (argument_name(source, prev), argument_name(source, curr)) else {
        return;
    };
    if correct_order(&pn, &cn, order_ref, source) {
        return;
    }
    let (line, col) = source.offset_to_line_col(curr.start_byte());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        col,
        format!(
            "Arguments should be sorted in an alphabetical order within their section. Field `{cn}` should appear before `{pn}`."
        ),
    ));
}

fn collect_args<'a>(
    source: &SourceFile,
    node: Node<'a>,
    class_node: Node<'a>,
    out: &mut Vec<Node<'a>>,
) {
    if node.kind() == "class" && node.id() != class_node.id() {
        return;
    }
    if is_argument_call(source, node)
        && enclosing_class(node).is_some_and(|c| c.id() == class_node.id())
    {
        out.push(node);
    }
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        collect_args(source, child, class_node, out);
    }
}
