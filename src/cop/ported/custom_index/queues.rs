use super::*;

#[derive(Default)]
struct Queue {
    worker: bool,
    declared: bool,
}
#[derive(Default)]
pub(super) struct Queues(HashMap<Key, Queue>);
impl Queues {
    pub(super) fn new(c: &Context<'_>, root: Node<'_>) -> Self {
        let mut result = Self::default();
        result.walk(c, root, None, None);
        result
    }
    pub(super) fn get(&self, n: Node<'_>) -> (bool, bool) {
        self.0
            .get(&key(n))
            .map_or((false, false), |q| (q.worker, q.declared))
    }
    fn walk(&mut self, c: &Context<'_>, n: Node<'_>, owner: Option<Key>, excluded: Option<Key>) {
        if Some(key(n)) != excluded && is_call(n) {
            self.call(c, n, owner);
        }
        let (owner, excluded) = if matches!(n.kind(), "class" | "module") {
            (Some(key(n)), single_statement(n))
        } else {
            (owner, excluded)
        };
        let mut cursor = n.walk();
        for child in n.named_children(&mut cursor) {
            self.walk(c, child, owner, excluded);
        }
    }
    fn call(&mut self, c: &Context<'_>, n: Node<'_>, owner: Option<Key>) {
        if !matches!(
            c.method(n),
            "include" | "sidekiq_options" | "use_default_queue!"
        ) {
            return;
        }
        if let Some(owner) = owner {
            update(c, n, self.0.entry(owner).or_default());
        }
    }
}
fn single_statement(n: Node<'_>) -> Option<Key> {
    let statements = body(n).map(statements).unwrap_or_default();
    (statements.len() == 1).then(|| key(statements[0]))
}
fn update(c: &Context<'_>, n: Node<'_>, queue: &mut Queue) {
    match c.method(n) {
        "include" => queue.worker |= args(n).iter().any(|a| c.text(*a) == "Sidekiq::Worker"),
        "use_default_queue!" => queue.declared = true,
        "sidekiq_options" => queue.declared |= c.pair_value(n, "queue").is_some(),
        _ => {}
    }
}
