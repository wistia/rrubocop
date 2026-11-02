use super::super::context::*;
use tree_sitter::Node;
pub(super) fn top_method(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if is_call(n) && c.method(n) != "define_method" {
        return vec![];
    }
    if root_statement(n) {
        vec![Finding::new(n, "Do not define methods at the top-level.")]
    } else {
        vec![]
    }
}

fn root_statement(n: Node<'_>) -> bool {
    n.parent().is_some_and(|parent| {
        parent.kind() == "program"
            || parent.kind() == "parenthesized_statements"
                && parent
                    .parent()
                    .is_some_and(|program| program.kind() == "program" && kids(program).len() == 1)
    })
}
