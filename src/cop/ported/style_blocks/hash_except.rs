use super::super::context::*;
use tree_sitter::Node;
pub(super) fn hash_except(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if safe(c, n) && c.config.get_bool("ActiveSupportExtensionsEnabled", false) {
        return vec![];
    }
    if !except_candidate(c, n) {
        return vec![];
    }
    let Some((except, kind)) = block_key(c, n) else {
        return vec![];
    };
    if (c.method(n) == "reject") != kind {
        return vec![];
    }
    let preferred = format!("except({})", key_content(c, except));
    let start = method_node(n).start_byte();
    vec![
        Finding::range(start, n.end_byte(), format!("Use `{preferred}` instead.")).edit_with(
            c,
            start,
            n.end_byte(),
            || preferred,
        ),
    ]
}
fn except_candidate(c: &Context<'_>, n: Node<'_>) -> bool {
    ["reject", "select", "filter"].contains(&c.method(n))
        && args(n).is_empty()
        && c.config.get_f64("TargetRubyVersion", 2.7) >= 3.0
}
fn block_key<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, bool)> {
    let (expression, param) = block_expression(n)?;
    let negated = expression.kind() == "unary" && operator(c, expression) == "!";
    if invalid_negation(c, n, negated) {
        return None;
    }
    let bare = if negated {
        field(expression, "operand")?
    } else {
        expression
    };
    let (key, kind) = except_key(c, bare, c.text(param))?;
    Some((key, negated_kind(c, n, bare, negated, kind)))
}
fn block_expression(n: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    let b = block(n)?;
    let p = field(b, "parameters").map(kids).unwrap_or_default();
    if p.len() != 2 {
        return None;
    }
    Some((statements(body(b)?).first().copied()?, p[0]))
}
fn invalid_negation(c: &Context<'_>, n: Node<'_>, negated: bool) -> bool {
    negated
        && c.method(n) == "reject"
        && !c.config.get_bool("ActiveSupportExtensionsEnabled", false)
}
fn negated_kind(c: &Context<'_>, n: Node<'_>, bare: Node<'_>, negated: bool, kind: bool) -> bool {
    if negated && ["include?", "in?", "exclude?"].contains(&c.method(bare)) {
        c.method(n) == "reject"
    } else {
        kind
    }
}
mod content;
mod matcher;
use content::key_content;
use matcher::except_key;
