use super::super::context::*;
use tree_sitter::Node;

pub(super) fn sidekiq_receive(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !receives_async(c, n) || safe_ancestor(c, n) {
        return vec![];
    }
    vec![Finding::new(
        n,
        "receive(:perform_async) called without safe ancestor method. This could have test mask passing incorrect arguments to a worker.",
    )]
}

fn receives_async(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "receive"
        && args(n)
            .first()
            .is_some_and(|argument| async_symbol(c, *argument))
}

fn async_symbol(c: &Context<'_>, n: Node<'_>) -> bool {
    c.symbol(n) && ["perform_async", "perform_in", "perform_at"].contains(&c.value(n))
}

fn safe_ancestor(c: &Context<'_>, n: Node<'_>) -> bool {
    ancestors(n)
        .into_iter()
        .take_while(|parent| is_call(*parent) || parent.kind() == "argument_list")
        .any(|parent| {
            ["and_call_original", "and_wrap_original", "never", "not_to"]
                .contains(&c.method(parent))
        })
}
