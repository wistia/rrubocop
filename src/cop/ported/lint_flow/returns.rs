use super::super::context::*;
use tree_sitter::Node;
pub(super) fn void_return(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if kids(n).is_empty() {
        return vec![];
    }
    let Some(scope) = ancestors(n).into_iter().find(|p| return_scope(*p)) else {
        return vec![];
    };
    let name = c.method(scope);
    if scope.kind() != "method" || !void_method(name) {
        return vec![];
    }
    vec![Finding::range(
        n.start_byte(),
        n.start_byte() + 6,
        format!("Do not return a value in `{name}`."),
    )]
}
fn return_scope(n: Node<'_>) -> bool {
    matches!(
        n.kind(),
        "method" | "singleton_method" | "block" | "do_block" | "lambda"
    )
}
fn void_method(name: &str) -> bool {
    name == "initialize"
        || name.ends_with('=') && !matches!(name, "==" | "===" | "!=" | "<=" | ">=")
}
pub(super) fn top_return(_c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if kids(n).is_empty() || ancestors(n).iter().any(|p| return_scope(*p)) {
        return vec![];
    }
    vec![Finding::new(n, "Top level return with argument detected.").replace(n, "return")]
}
