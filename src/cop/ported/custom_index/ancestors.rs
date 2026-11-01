use super::*;
use std::sync::Mutex;

#[derive(Clone, Copy, Default)]
struct Flags {
    class: bool,
    spec: bool,
}
#[derive(Default)]
struct Contexts(Mutex<HashMap<Key, Flags>>);
pub(in crate::cop::ported) fn in_class(c: &Context<'_>, n: Node<'_>) -> bool {
    ancestors(c, n).class
}
pub(in crate::cop::ported) fn in_spec(c: &Context<'_>, n: Node<'_>) -> bool {
    ancestors(c, n).spec
}
fn ancestors(c: &Context<'_>, n: Node<'_>) -> Flags {
    let cache = c.source.cached(0, Contexts::default);
    let mut flags = cache.0.lock().unwrap();
    n.parent()
        .map_or_else(Flags::default, |parent| including(c, parent, &mut flags))
}
fn including(c: &Context<'_>, n: Node<'_>, cache: &mut HashMap<Key, Flags>) -> Flags {
    if let Some(flags) = cache.get(&key(n)) {
        return *flags;
    }
    let parent = n
        .parent()
        .map_or_else(Flags::default, |p| including(c, p, cache));
    let flags = Flags {
        class: parent.class || n.kind() == "class",
        spec: parent.spec || spec_group(c, n),
    };
    cache.insert(key(n), flags);
    flags
}
fn spec_group(c: &Context<'_>, n: Node<'_>) -> bool {
    block(n).is_some()
        && ["", "RSpec", "::RSpec"].contains(&c.receiver(n))
        && crate::cop::rspec::helpers::is_group(c.method(n).as_bytes())
}
