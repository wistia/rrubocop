use super::*;
use std::collections::HashMap;
use std::sync::Mutex;

type Key = (usize, usize, &'static str);
pub(super) struct Reporters(Option<Mutex<HashMap<Key, bool>>>);
impl Reporters {
    pub(super) fn new(c: &Context<'_>, _n: Node<'_>) -> Self {
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
        Self(Some(Mutex::default()))
    }
    pub(super) fn contains(&self, c: &Context<'_>, n: Node<'_>) -> bool {
        match &self.0 {
            Some(found) => summarize(c, n, &mut found.lock().unwrap(), true),
            None => descendants(n).any(|node| reports(c, node)),
        }
    }
}
fn key(n: Node<'_>) -> Key {
    (n.start_byte(), n.end_byte(), n.kind())
}
fn summarize(c: &Context<'_>, n: Node<'_>, found: &mut HashMap<Key, bool>, cache: bool) -> bool {
    if cache && let Some(reporting) = found.get(&key(n)) {
        return *reporting;
    }
    let mut cursor = n.walk();
    let reporting = reports(c, n)
        || n.named_children(&mut cursor)
            .any(|child| summarize(c, child, found, handler(n, child)));
    if cache {
        found.insert(key(n), reporting);
    }
    reporting
}

fn handler(parent: Node<'_>, child: Node<'_>) -> bool {
    match parent.kind() {
        "rescue" => body(parent) == Some(child),
        "rescue_modifier" => field(parent, "handler") == Some(child),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
