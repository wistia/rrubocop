use super::super::context::*;
use super::policies::superclass;
use tree_sitter::Node;

pub(super) fn queue(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let parent = superclass(c, n);
    if parent.contains("Nsq::BaseWorker") || parent == "NsqWorker::Worker" {
        return vec![];
    }
    if complete(c, n, parent) {
        return vec![];
    }
    vec![Finding::new(
        n,
        "Sidekiq workers must explicitly declare their queue. Use `sidekiq_options queue: \"queue_name\"` or `use_default_queue!` to opt into the default queue.",
    )]
}

fn complete(c: &Context<'_>, n: Node<'_>, parent: &str) -> bool {
    let known = parent == "BaseWorker" || parent.ends_with("::BaseWorker");
    if known && direct_queue(c, n) {
        return true;
    }
    let (included, declared) = super::super::custom_index::index(c, n).worker_queue(n);
    !(known || included) || declared
}
fn direct_queue(c: &Context<'_>, n: Node<'_>) -> bool {
    let statements = body(n).map(statements).unwrap_or_default();
    statements.len() > 1
        && statements.into_iter().any(|n| {
            is_call(n)
                && (c.method(n) == "use_default_queue!"
                    || c.method(n) == "sidekiq_options" && c.pair_value(n, "queue").is_some())
        })
}
