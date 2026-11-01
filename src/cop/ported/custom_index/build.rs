use super::*;

#[derive(Clone, Copy, Default)]
struct Owners {
    method: Option<Key>,
    class: Option<Key>,
    policy: Option<Key>,
    excluded: Option<Key>,
}
pub(super) fn index(c: &Context<'_>, root: Node<'_>) -> Index {
    let mut builder = Builder {
        c,
        result: Index::default(),
        next: 0,
    };
    builder.walk(root, None, Owners::default());
    builder.result
}
struct Builder<'a, 's> {
    c: &'a Context<'s>,
    result: Index,
    next: usize,
}
impl Builder<'_, '_> {
    fn walk(&mut self, n: Node<'_>, parent: Option<Key>, owners: Owners) {
        let start = self.next;
        self.next += 1;
        self.summarize(n, start, owners);
        let nested = nested_owners(self.c, n, owners);
        let mut cursor = n.walk();
        for child in n.named_children(&mut cursor) {
            self.walk(child, Some(key(n)), nested);
        }
        self.result.records.insert(
            key(n),
            Record {
                range: (start, self.next),
                parent,
                owner: owners.method,
                policy: owners.policy,
            },
        );
    }
    fn summarize(&mut self, n: Node<'_>, ordinal: usize, owners: Owners) {
        if n.kind() == "method" {
            self.result
                .methods
                .entry(self.c.method(n).into())
                .or_default()
                .push(ordinal);
            if let Some(owner) = owners.method {
                self.result
                    .owned_methods
                    .entry(owner)
                    .or_default()
                    .insert(self.c.method(n).into());
            }
        }
        if is_call(n) {
            self.call(n, ordinal, owners);
        }
        if scope_class(self.c, n) {
            self.result.scopes.push(ordinal);
        }
    }
    fn call(&mut self, n: Node<'_>, ordinal: usize, owners: Owners) {
        let name = self.c.method(n);
        if name == "disable_scoping!" {
            self.result.disabled.push(ordinal);
        }
        if Some(key(n)) == owners.excluded {
            return;
        }
        if !matches!(name, "include" | "sidekiq_options" | "use_default_queue!") {
            return;
        }
        if let Some(owner) = owners.class {
            let queue = self.result.queues.entry(owner).or_default();
            update_queue(self.c, n, queue);
        }
    }
}
fn nested_owners(c: &Context<'_>, n: Node<'_>, mut owners: Owners) -> Owners {
    if matches!(n.kind(), "class" | "module" | "singleton_class") {
        owners.method = Some(key(n));
    }
    if matches!(n.kind(), "class" | "module") {
        owners.class = Some(key(n));
        owners.excluded = excluded(n);
    }
    if n.kind() == "class" && field(n, "name").is_some_and(|name| c.text(name).ends_with("Policy"))
    {
        owners.policy = Some(key(n));
    }
    owners
}
fn excluded(n: Node<'_>) -> Option<Key> {
    let statements = body(n).map(statements).unwrap_or_default();
    (statements.len() == 1).then(|| key(statements[0]))
}
fn scope_class(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "class"
        && matches!(
            superclass(c, n),
            "ApplicationPolicy::Scope" | "JwtPolicy::Scope"
        )
}
fn superclass<'a>(c: &Context<'a>, n: Node<'_>) -> &'a str {
    field(n, "superclass")
        .and_then(|s| s.named_child(0))
        .map(|s| c.text(s).trim_start_matches("::"))
        .unwrap_or("")
}

fn update_queue(c: &Context<'_>, n: Node<'_>, queue: &mut Queue) {
    match c.method(n) {
        "include" => queue.worker |= args(n).iter().any(|a| c.text(*a) == "Sidekiq::Worker"),
        "use_default_queue!" => queue.declared = true,
        "sidekiq_options" => queue.declared |= c.pair_value(n, "queue").is_some(),
        _ => {}
    }
}
