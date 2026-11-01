use super::context::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use tree_sitter::Node;

mod ancestors;
mod methods;
mod policies;
mod queues;
pub(super) use ancestors::{in_class, in_spec};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct Key(usize, usize, &'static str);
fn key(n: Node<'_>) -> Key {
    Key(n.start_byte(), n.end_byte(), n.kind())
}
fn root(mut n: Node<'_>) -> Node<'_> {
    while let Some(parent) = n.parent() {
        n = parent;
    }
    n
}
#[derive(Default)]
pub(super) struct Index {
    methods: OnceLock<methods::Methods>,
    queues: OnceLock<queues::Queues>,
    policies: OnceLock<policies::Policies>,
    worker_marker: OnceLock<bool>,
}
pub(super) fn index(c: &Context<'_>, _n: Node<'_>) -> Arc<Index> {
    c.source.cached(0, Index::default)
}
impl Index {
    pub(super) fn method(&self, c: &Context<'_>, n: Node<'_>, name: &str, strict: bool) -> bool {
        self.methods
            .get_or_init(|| methods::Methods::new(c, root(n)))
            .contains(n, name, strict)
    }
    pub(super) fn may_include_worker(&self, c: &Context<'_>) -> bool {
        *self.worker_marker.get_or_init(|| {
            c.source
                .as_bytes()
                .windows(b"Sidekiq".len())
                .any(|w| w == b"Sidekiq")
        })
    }
    pub(super) fn worker_queue(&self, c: &Context<'_>, n: Node<'_>) -> (bool, bool) {
        self.queues
            .get_or_init(|| queues::Queues::new(c, root(n)))
            .get(n)
    }
    fn policies(&self, c: &Context<'_>, n: Node<'_>) -> &policies::Policies {
        self.policies
            .get_or_init(|| policies::Policies::new(c, root(n)))
    }
    pub(super) fn policy_disabled(&self, c: &Context<'_>, n: Node<'_>) -> bool {
        self.policies(c, n).disabled(n)
    }
    pub(super) fn policy_scope(&self, c: &Context<'_>, n: Node<'_>) -> bool {
        self.policies(c, n).scope(n)
    }
    pub(super) fn parent_policy_disabled(&self, c: &Context<'_>, n: Node<'_>) -> Option<bool> {
        self.policies(c, n).parent_disabled(n)
    }
}
fn in_range(positions: &[usize], (start, end): (usize, usize)) -> bool {
    positions
        .get(positions.partition_point(|p| *p < start))
        .is_some_and(|p| *p < end)
}

#[cfg(test)]
mod tests;
