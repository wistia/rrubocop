use super::super::context::*;
use tree_sitter::Node;

const MESSAGE: &str = "Rescue blocks must report the error using `Sentry.capture_exception`, `Sentry.capture_message`, or `Rails.error.report`";

pub(super) fn report_error(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if n.kind() == "rescue_modifier" {
        return modifier(c, n);
    }
    if !broad_rescue(c, n) || body(n).is_some_and(|b| reports_in(c, b)) {
        return vec![];
    }
    vec![Finding::range(n.start_byte(), rescue_end(n), MESSAGE)]
}

fn rescue_end(n: Node<'_>) -> usize {
    body(n)
        .and_then(|body| {
            statements(body)
                .into_iter()
                .rfind(|node| node.kind() != "empty_statement")
        })
        .map_or(n.end_byte(), |last| last.end_byte())
}

fn modifier(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if field(n, "handler").is_some_and(|handler| reports_in(c, handler)) {
        return vec![];
    }
    let mut cursor = n.walk();
    let Some(keyword) = n
        .children(&mut cursor)
        .find(|child| child.kind() == "rescue")
    else {
        return vec![];
    };
    vec![Finding::range(keyword.start_byte(), n.end_byte(), MESSAGE)]
}

fn broad_rescue(c: &Context<'_>, n: Node<'_>) -> bool {
    n.child_by_field_name("exceptions")
        .is_none_or(|exceptions| {
            kids(exceptions).iter().any(|exception| {
                c.constant_name(*exception)
                    .is_some_and(|name| ["StandardError", "Exception"].contains(&name.as_ref()))
            })
        })
}

fn reports_in(c: &Context<'_>, n: Node<'_>) -> bool {
    let summary = c.source.cached(0, || summary::Reporters::new(c, n));
    summary.contains(c, n)
}

fn reports(c: &Context<'_>, n: Node<'_>) -> bool {
    if is_call(n) && !safe(c, n) {
        c.method(n) == "raise" || sentry_report(c, n) || rails_report(c, n)
    } else {
        bare_raise(c, n)
    }
}

fn bare_raise(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "identifier"
        && c.text(n) == "raise"
        && !n
            .parent()
            .is_some_and(|p| field(p, "method") == Some(n) || field(p, "name") == Some(n))
        && !local_identifier(c, n)
}

fn sentry_report(c: &Context<'_>, n: Node<'_>) -> bool {
    field(n, "receiver").is_some_and(|r| constant(c, r, "Sentry"))
        && ["capture_exception", "capture_message"].contains(&c.method(n))
}

fn rails_report(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "report" && field(n, "receiver").is_some_and(|receiver| rails_error(c, receiver))
}

fn rails_error(c: &Context<'_>, n: Node<'_>) -> bool {
    is_call(n)
        && !safe(c, n)
        && c.method(n) == "error"
        && field(n, "receiver").is_some_and(|receiver| constant(c, receiver, "Rails"))
}

fn constant(c: &Context<'_>, n: Node<'_>, name: &str) -> bool {
    n.kind() == "constant" && c.text(n) == name
        || n.kind() == "scope_resolution"
            && field(n, "scope").is_none()
            && field(n, "name").is_some_and(|constant| c.text(constant) == name)
}

mod summary;
