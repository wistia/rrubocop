use super::*;
use std::collections::{HashMap, HashSet};

type Key = (usize, usize, &'static str);
pub(super) struct Transactions(HashSet<Key>);
impl Transactions {
    pub(super) fn new(c: &Context<'_>, n: Node<'_>) -> Self {
        let mut root = n;
        while let Some(parent) = root.parent() {
            root = parent;
        }
        let mut builder = Builder {
            c,
            bodies: HashMap::new(),
            calls: HashSet::new(),
        };
        builder.walk(root, [None; 3]);
        Self(builder.calls)
    }
    pub(super) fn contains(&self, n: Node<'_>) -> bool {
        self.0.contains(&key(n))
    }
}
fn key(n: Node<'_>) -> Key {
    (n.start_byte(), n.end_byte(), n.kind())
}
struct Builder<'a, 's> {
    c: &'a Context<'s>,
    bodies: HashMap<Key, Option<String>>,
    calls: HashSet<Key>,
}
impl<'n> Builder<'_, '_> {
    fn walk(&mut self, n: Node<'n>, parents: [Option<Node<'n>>; 3]) {
        if is_call(n) && self.c.method(n) == "set" {
            self.set(n, parents);
        }
        let mut cursor = n.walk();
        for child in n.named_children(&mut cursor) {
            self.walk(child, [Some(n), parents[0], parents[1]]);
        }
    }
    fn set(&mut self, n: Node<'_>, parents: [Option<Node<'_>>; 3]) {
        let [Some(body), _, Some(call)] = parents else {
            return;
        };
        let parameter = self.bodies.entry(key(body)).or_insert_with(|| {
            if transaction_body(body) && transaction(self.c, call) {
                first_parameter(call).map(|p| self.c.text(p).to_owned())
            } else {
                None
            }
        });
        if parameter.as_deref() == Some(self.c.receiver(n)) {
            self.calls.insert(key(n));
        }
    }
}
