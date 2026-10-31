use super::super::context::*;
use super::policies::superclass;
use tree_sitter::Node;

pub(super) fn queue(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let parent = superclass(c, n);
    if parent.contains("Nsq::BaseWorker") || parent == "NsqWorker::Worker" {
        return vec![];
    }
    let sends = class_calls(n);
    if !worker(c, parent, &sends) || sends.iter().any(|call| queue_declaration(c, *call)) {
        return vec![];
    }
    vec![Finding::new(
        n,
        "Sidekiq workers must explicitly declare their queue. Use `sidekiq_options queue: \"queue_name\"` or `use_default_queue!` to opt into the default queue.",
    )]
}

fn class_calls(n: Node<'_>) -> Vec<Node<'_>> {
    let statements = body(n).map(statements).unwrap_or_default();
    let excluded = (statements.len() == 1).then(|| statements[0]);
    descendants(n)
        .filter(|d| Some(*d) != excluded)
        .filter(|d| belongs_to_class(*d, n))
        .collect()
}

fn belongs_to_class(n: Node<'_>, class: Node<'_>) -> bool {
    is_call(n) && ancestors(n).find(|p| matches!(p.kind(), "class" | "module")) == Some(class)
}

fn worker(c: &Context<'_>, parent: &str, sends: &[Node<'_>]) -> bool {
    parent == "BaseWorker"
        || parent.ends_with("::BaseWorker")
        || sends.iter().any(|call| includes_worker(c, *call))
}

fn includes_worker(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "include"
        && args(n)
            .iter()
            .any(|argument| c.text(*argument) == "Sidekiq::Worker")
}

fn queue_declaration(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "use_default_queue!"
        || c.method(n) == "sidekiq_options" && c.pair_value(n, "queue").is_some()
}
