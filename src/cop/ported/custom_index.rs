use super::context::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tree_sitter::Node;

mod ancestors;
mod build;
pub(super) use ancestors::{in_class, in_spec};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct Key(usize, usize, &'static str);
fn key(n: Node<'_>) -> Key {
    Key(n.start_byte(), n.end_byte(), n.kind())
}
#[derive(Clone, Copy)]
struct Record {
    range: (usize, usize),
    parent: Option<Key>,
    owner: Option<Key>,
    policy: Option<Key>,
}
#[derive(Default)]
struct Queue {
    worker: bool,
    declared: bool,
}
#[derive(Default)]
pub(super) struct Index {
    records: HashMap<Key, Record>,
    methods: HashMap<String, Vec<usize>>,
    owned_methods: HashMap<Key, HashSet<String>>,
    queues: HashMap<Key, Queue>,
    disabled: Vec<usize>,
    scopes: Vec<usize>,
}
pub(super) fn index(c: &Context<'_>, n: Node<'_>) -> Arc<Index> {
    c.source.cached(0, || {
        let mut root = n;
        while let Some(parent) = root.parent() {
            root = parent;
        }
        build::index(c, root)
    })
}
impl Index {
    pub(super) fn method(&self, n: Node<'_>, name: &str, strict: bool) -> bool {
        let Some(record) = self.records.get(&key(n)) else {
            return false;
        };
        if strict {
            return record.owner.is_some_and(|owner| {
                self.owned_methods
                    .get(&owner)
                    .is_some_and(|methods| methods.contains(name))
            });
        }
        let parent = record.parent.and_then(|p| self.records.get(&p));
        parent.is_some_and(|p| {
            self.methods
                .get(name)
                .is_some_and(|positions| in_range(positions, p.range))
        })
    }
    pub(super) fn worker_queue(&self, n: Node<'_>) -> (bool, bool) {
        self.queues
            .get(&key(n))
            .map_or((false, false), |q| (q.worker, q.declared))
    }
    fn contains(&self, n: Node<'_>, positions: &[usize]) -> bool {
        self.records
            .get(&key(n))
            .is_some_and(|r| in_range(positions, r.range))
    }
    pub(super) fn policy_disabled(&self, n: Node<'_>) -> bool {
        self.contains(n, &self.disabled)
    }
    pub(super) fn policy_scope(&self, n: Node<'_>) -> bool {
        self.contains(n, &self.scopes)
    }
    pub(super) fn parent_policy_disabled(&self, n: Node<'_>) -> Option<bool> {
        self.records
            .get(&key(n))?
            .policy
            .map(|policy| in_range(&self.disabled, self.records[&policy].range))
    }
}
fn in_range(positions: &[usize], (start, end): (usize, usize)) -> bool {
    positions
        .get(positions.partition_point(|p| *p < start))
        .is_some_and(|p| *p < end)
}

#[cfg(test)]
mod tests;
