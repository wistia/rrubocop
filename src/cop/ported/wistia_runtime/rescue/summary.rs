use super::*;
use std::collections::HashSet;

type Key = (usize, usize, &'static str);
pub(super) struct Reporters(Option<HashSet<Key>>);
impl Reporters {
    pub(super) fn new(c: &Context<'_>, n: Node<'_>) -> Self {
        if c.source
            .as_bytes()
            .windows(6)
            .filter(|w| *w == b"rescue")
            .take(2)
            .count()
            < 2
        {
            return Self(None);
        }
        let mut root = n;
        while let Some(parent) = root.parent() {
            root = parent;
        }
        let mut found = HashSet::new();
        summarize(c, root, &mut found);
        Self(Some(found))
    }
    pub(super) fn contains(&self, c: &Context<'_>, n: Node<'_>) -> bool {
        match &self.0 {
            Some(found) => found.contains(&key(n)),
            None => descendants(n).any(|node| reports(c, node)),
        }
    }
}
fn key(n: Node<'_>) -> Key {
    (n.start_byte(), n.end_byte(), n.kind())
}
fn summarize(c: &Context<'_>, n: Node<'_>, found: &mut HashSet<Key>) -> bool {
    let mut reporting = reports(c, n);
    let mut cursor = n.walk();
    for child in n.named_children(&mut cursor) {
        reporting |= summarize(c, child, found);
    }
    if reporting {
        found.insert(key(n));
    }
    reporting
}
