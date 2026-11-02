use super::*;

pub(super) fn block(c: &Context<'_>, n: Node<'_>) -> bool {
    c.config.get_f64("TargetRubyVersion", 3.4) >= 2.7
        && field(n, "parameters").is_none()
        && body(n).is_some_and(|body| contains_parameter(c, body))
}

fn contains_parameter(c: &Context<'_>, n: Node<'_>) -> bool {
    parameter(c, n)
        || lexical_children(n)
            .into_iter()
            .any(|child| contains_parameter(c, child))
}

fn lexical_children(n: Node<'_>) -> Vec<Node<'_>> {
    match n.kind() {
        "class" | "module" => ["name", "superclass"]
            .into_iter()
            .filter_map(|name| field(n, name))
            .collect(),
        "singleton_method" => field(n, "object").into_iter().collect(),
        "singleton_class" => field(n, "value").into_iter().collect(),
        "block" | "do_block" | "lambda" | "method" => vec![],
        _ => kids(n),
    }
}

fn parameter(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "identifier"
        && matches!(c.text(n).as_bytes(), [b'_', b'1'..=b'9'])
        && n.parent()
            .is_none_or(|parent| field(parent, "method") != Some(n))
}
