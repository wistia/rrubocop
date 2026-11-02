use super::*;
use std::sync::Mutex;

const INDEX_AFTER: usize = 32;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Flags {
    class: bool,
    spec: bool,
}
#[derive(Default)]
struct Contexts(Mutex<State>);
#[derive(Default)]
struct State {
    queries: HashMap<Key, Flags>,
    parents: HashMap<Key, Flags>,
    indexed: bool,
}

pub(in crate::cop::ported) fn in_class(c: &Context<'_>, n: Node<'_>) -> bool {
    ancestors(c, n).class
}
pub(in crate::cop::ported) fn in_spec(c: &Context<'_>, n: Node<'_>) -> bool {
    ancestors(c, n).spec
}
fn ancestors(c: &Context<'_>, n: Node<'_>) -> Flags {
    let cache = c.source.cached(0, Contexts::default);
    cache.0.lock().unwrap().query(c, n)
}
impl State {
    fn query(&mut self, c: &Context<'_>, n: Node<'_>) -> Flags {
        if let Some(flags) = self.queries.get(&key(n)) {
            return *flags;
        }
        if !self.indexed && self.queries.len() >= INDEX_AFTER {
            self.index(c, root(n));
        }
        if let Some(flags) = self.queries.get(&key(n)) {
            return *flags;
        }
        let flags = n.parent().map_or_else(Flags::default, |parent| {
            including(c, parent, &mut self.parents)
        });
        self.queries.insert(key(n), flags);
        flags
    }
    fn index(&mut self, c: &Context<'_>, root: Node<'_>) {
        self.indexed = true;
        self.parents = HashMap::new();
        let mut cursor = root.walk();
        let mut inherited = vec![Flags::default()];
        loop {
            let node = cursor.node();
            let parent = *inherited.last().unwrap();
            if node.is_named() {
                self.queries.insert(key(node), parent);
            }
            if cursor.goto_first_child() {
                inherited.push(including_self(c, node, parent));
                continue;
            }
            if !advance(&mut cursor, &mut inherited) {
                return;
            }
        }
    }
}
fn advance(cursor: &mut tree_sitter::TreeCursor<'_>, inherited: &mut Vec<Flags>) -> bool {
    loop {
        if cursor.goto_next_sibling() {
            return true;
        }
        if !cursor.goto_parent() {
            return false;
        }
        inherited.pop();
    }
}

fn including(c: &Context<'_>, n: Node<'_>, cache: &mut HashMap<Key, Flags>) -> Flags {
    if let Some(flags) = cache.get(&key(n)) {
        return *flags;
    }
    let parent = n
        .parent()
        .map_or_else(Flags::default, |p| including(c, p, cache));
    let flags = including_self(c, n, parent);
    cache.insert(key(n), flags);
    flags
}
fn including_self(c: &Context<'_>, n: Node<'_>, parent: Flags) -> Flags {
    Flags {
        class: parent.class || n.kind() == "class",
        spec: parent.spec || spec_group(c, n),
    }
}
fn spec_group(c: &Context<'_>, n: Node<'_>) -> bool {
    block(n).is_some()
        && ["", "RSpec", "::RSpec"].contains(&c.receiver(n))
        && crate::cop::rspec::helpers::is_group(c.method(n).as_bytes())
        && block(n).is_none_or(|block| !numbered::block(c, block))
}

mod numbered;

#[cfg(test)]
mod tests;
