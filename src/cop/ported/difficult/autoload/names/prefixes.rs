use super::*;
use std::collections::HashMap;
use std::sync::Mutex;
mod state;
pub(super) use state::Namespace;

type State = Option<Namespace>;
type Key = (usize, usize, u16);
#[derive(Default)]
struct Prefixes(Mutex<HashMap<Key, State>>);

pub(super) fn parent(c: &Context<'_>, node: Node<'_>, limit: usize) -> State {
    let prefixes = c.source.cached(limit, Prefixes::default);
    node.parent().map_or_else(
        || Some(Namespace::default()),
        |parent| after(c, parent, &prefixes, limit),
    )
}
fn after(c: &Context<'_>, node: Node<'_>, prefixes: &Prefixes, limit: usize) -> State {
    let key = (node.start_byte(), node.end_byte(), node.kind_id());
    if let Some(state) = prefixes.0.lock().unwrap().get(&key).cloned() {
        return state;
    }
    let state = extend(c, node, inherited(c, node, prefixes, limit), limit);
    prefixes.0.lock().unwrap().insert(key, state.clone());
    state
}
fn inherited(c: &Context<'_>, node: Node<'_>, prefixes: &Prefixes, limit: usize) -> State {
    node.parent().map_or_else(
        || Some(Namespace::default()),
        |parent| after(c, parent, prefixes, limit),
    )
}
fn extend(c: &Context<'_>, node: Node<'_>, inherited: State, limit: usize) -> State {
    let prefix = inherited?;
    let mut parts = Vec::new();
    if singleton_self(c, node) {
        parts.push(format!("#<Class:{}>", prefix.full_name()));
    } else {
        add_namespace(c, node, &mut parts)?;
    }
    Some(match parts.pop() {
        Some(part) => prefix.append(part, limit),
        None => prefix,
    })
}
fn singleton_self(c: &Context<'_>, node: Node<'_>) -> bool {
    node.kind() == "singleton_class"
        && field(node, "value")
            .or_else(|| kids(node).first().copied())
            .is_some_and(|object| c.text(object) == "self")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::parse::source::SourceFile;

    #[test]
    fn memoized_prefixes_preserve_all_ancestor_namespace_rules() {
        for text in [
            "module Outer; class Inner; CONST=1; end; end",
            "Foo=Class.new do; Bar=Module.new do; VALUE=1; end; end",
            "Outer.class_eval do; class Inner; end; end",
            "module Outer; class << self; VALUE=1; end; end",
            "class << Outer; VALUE=1; end",
            "items.each do; class Thing; end; end",
            "dynamic.class_eval { VALUE=1 }",
            "module Outer; class ::Inner; VALUE=1; end; end",
        ] {
            compare_namespace_prefixes(text);
        }
    }
    fn compare_namespace_prefixes(text: &str) {
        let source = SourceFile::from_bytes("example.rb", text.as_bytes().to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let c = Context::new(&source, &config);
        for node in descendants(tree.root_node()) {
            assert_eq!(
                parent(&c, node, usize::MAX).map(|p| p.full_name()),
                replay(&c, node),
                "{} {text}",
                node.kind()
            );
        }
    }
    fn replay(c: &Context<'_>, node: Node<'_>) -> Option<String> {
        let mut parts = Vec::new();
        for ancestor in ancestors(node).collect::<Vec<_>>().into_iter().rev() {
            add_namespace(c, ancestor, &mut parts)?;
        }
        Some(scope_name(&parts))
    }
}
