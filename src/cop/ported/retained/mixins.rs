use super::super::context::*;
use super::arguments as args;
use tree_sitter::Node;

pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(body) = field(n, "body") else {
        return vec![];
    };
    let calls = body_statements(body)
        .into_iter()
        .filter(|ch| mixin_call(c, *ch))
        .collect::<Vec<_>>();
    ["include", "extend", "prepend"]
        .into_iter()
        .flat_map(|method| check_method(c, &calls, method))
        .collect()
}
fn mixin_call(c: &Context<'_>, n: Node<'_>) -> bool {
    is_call(n) && c.receiver(n).is_empty() && block(n).is_none()
}
fn check_method(c: &Context<'_>, calls: &[Node<'_>], method: &str) -> Vec<Finding> {
    let mixins = method_calls(c, calls, method);
    let grouped = c.config.get_str("EnforcedStyle", "separated") == "grouped";
    if grouped && mixins.len() < 2 {
        return vec![];
    }
    mixins
        .into_iter()
        .filter(|n| flagged(*n, grouped))
        .map(|n| finding(n, method, grouped))
        .collect()
}
fn finding(n: Node<'_>, method: &str, grouped: bool) -> Finding {
    let style = if grouped {
        "a single statement"
    } else {
        "separate statements"
    };
    Finding::new(n, format!("Put `{method}` mixins in {style}."))
}

fn flagged(n: Node<'_>, grouped: bool) -> bool {
    if grouped {
        !args(n).is_empty()
    } else {
        args(n).len() > 1
    }
}

fn method_calls<'n>(c: &Context<'_>, calls: &[Node<'n>], method: &str) -> Vec<Node<'n>> {
    calls
        .iter()
        .copied()
        .filter(|n| c.method(*n) == method)
        .collect()
}

fn body_statements(body: Node<'_>) -> Vec<Node<'_>> {
    let nodes = statements(body);
    match nodes.as_slice() {
        [only] if only.kind() == "parenthesized_statements" => kids(*only),
        _ => nodes,
    }
}
