use super::super::context::*;
use tree_sitter::Node;
pub(super) fn mixin(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !mixin_call(c, n) || !top_scope(n) {
        return vec![];
    }
    vec![Finding::new(
        n,
        format!(
            "`{}` is used at the top level. Use inside `class` or `module`.",
            c.method(n)
        ),
    )]
}
fn mixin_call(c: &Context<'_>, n: Node<'_>) -> bool {
    if !["include", "extend", "prepend"].contains(&c.method(n)) || !c.receiver(n).is_empty() {
        return false;
    }
    let a = args(n);
    a.len() == 1 && matches!(a[0].kind(), "constant" | "scope_resolution")
}
fn top_scope(n: Node<'_>) -> bool {
    if ancestors(n)
        .iter()
        .any(|p| matches!(p.kind(), "block" | "do_block" | "lambda"))
    {
        return false;
    }
    n.parent().is_none_or(|p| {
        matches!(
            p.kind(),
            "program" | "begin" | "body_statement" | "then" | "else" | "method"
        )
    }) && !ancestors(n)
        .iter()
        .any(|p| matches!(p.kind(), "class" | "module" | "singleton_class"))
}
