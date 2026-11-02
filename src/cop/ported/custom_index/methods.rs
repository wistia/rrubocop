use super::*;

#[derive(Clone, Copy)]
struct Query {
    parent: Option<Key>,
    owner: Option<Key>,
}
#[derive(Default)]
pub(super) struct Methods {
    queries: HashMap<Key, Query>,
    ranges: HashMap<Key, (usize, usize)>,
    names: HashMap<String, Vec<usize>>,
    owned: HashMap<Key, HashSet<String>>,
}
impl Methods {
    pub(super) fn new(c: &Context<'_>, root: Node<'_>) -> Self {
        let mut builder = Builder {
            c,
            result: Self::default(),
            next: 0,
            parents: HashSet::new(),
        };
        builder.walk(root, None);
        builder.result
    }
    pub(super) fn contains(&self, n: Node<'_>, name: &str, strict: bool) -> bool {
        let Some(query) = self.queries.get(&key(n)) else {
            return false;
        };
        if strict {
            return query.owner.is_some_and(|owner| {
                self.owned
                    .get(&owner)
                    .is_some_and(|names| names.contains(name))
            });
        }
        query
            .parent
            .and_then(|p| self.ranges.get(&p))
            .is_some_and(|range| {
                self.names
                    .get(name)
                    .is_some_and(|positions| in_range(positions, *range))
            })
    }
}
struct Builder<'a, 's> {
    c: &'a Context<'s>,
    result: Methods,
    parents: HashSet<Key>,
    next: usize,
}
impl Builder<'_, '_> {
    fn walk(&mut self, n: Node<'_>, owner: Option<Key>) {
        let start = self.next;
        self.next += 1;
        self.record(n, owner, start);
        let nested = if matches!(n.kind(), "class" | "module" | "singleton_class") {
            Some(key(n))
        } else {
            owner
        };
        let mut cursor = n.walk();
        for child in n.named_children(&mut cursor) {
            self.walk(child, nested);
        }
        if self.parents.contains(&key(n)) {
            self.result.ranges.insert(key(n), (start + 1, self.next));
        }
    }
    fn record(&mut self, n: Node<'_>, owner: Option<Key>, ordinal: usize) {
        if n.kind() == "method" {
            self.method(n, owner, ordinal);
        }
        if is_call(n) && matches!(self.c.method(n), "attribute" | "attributes") {
            let parent = query_parent(n).map(key);
            self.result.queries.insert(key(n), Query { parent, owner });
            if let Some(parent) = parent {
                self.parents.insert(parent);
            }
        }
    }
    fn method(&mut self, n: Node<'_>, owner: Option<Key>, ordinal: usize) {
        self.result
            .names
            .entry(self.c.method(n).into())
            .or_default()
            .push(ordinal);
        if let Some(owner) = owner {
            self.result
                .owned
                .entry(owner)
                .or_default()
                .insert(self.c.method(n).into());
        }
    }
}

fn query_parent(n: Node<'_>) -> Option<Node<'_>> {
    let parent = n.parent()?;
    let parent =
        if matches!(parent.kind(), "body_statement" | "block_body") && kids(parent).len() == 1 {
            parent.parent().unwrap_or(parent)
        } else {
            parent
        };
    Some(if matches!(parent.kind(), "block" | "do_block") {
        parent.parent().unwrap_or(parent)
    } else {
        parent
    })
}
