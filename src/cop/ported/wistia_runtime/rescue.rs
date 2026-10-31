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
    vec![Finding::new(n, MESSAGE)]
}

fn modifier(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if field(n, "handler").is_some_and(|handler| reports_in(c, handler)) {
        return vec![];
    }
    let start = c.text(n).find("rescue").unwrap_or(0) + n.start_byte();
    vec![Finding::range(start, n.end_byte(), MESSAGE)]
}

fn broad_rescue(c: &Context<'_>, n: Node<'_>) -> bool {
    n.child_by_field_name("exceptions")
        .is_none_or(|exceptions| {
            kids(exceptions).iter().any(|exception| {
                ["StandardError", "Exception"]
                    .contains(&c.text(*exception).trim_start_matches("::"))
            })
        })
}

fn reports_in(c: &Context<'_>, n: Node<'_>) -> bool {
    descendants(n).iter().any(|d| reports(c, *d))
}

fn reports(c: &Context<'_>, n: Node<'_>) -> bool {
    (is_call(n) || n.kind() == "identifier")
        && (c.method(n) == "raise"
            || sentry_report(c, n)
            || c.receiver(n).trim_start_matches("::") == "Rails.error" && c.method(n) == "report")
}

fn sentry_report(c: &Context<'_>, n: Node<'_>) -> bool {
    c.receiver(n).trim_start_matches("::") == "Sentry"
        && ["capture_exception", "capture_message"].contains(&c.method(n))
}
