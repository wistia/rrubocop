use super::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type State = Option<Arc<Vec<String>>>;
type Key = (usize, usize, u16);
#[derive(Default)]
struct Prefixes(Mutex<HashMap<Key, State>>);

pub(super) fn parent(c: &Context<'_>, node: Node<'_>) -> Option<String> {
    let prefixes = c.source.cached(0, Prefixes::default);
    node.parent().map_or_else(
        || Some("Object".into()),
        |parent| after(c, parent, &prefixes).map(|parts| scope_name(&parts)),
    )
}

fn after(c: &Context<'_>, node: Node<'_>, prefixes: &Prefixes) -> State {
    let key = (node.start_byte(), node.end_byte(), node.kind_id());
    if let Some(state) = prefixes.0.lock().unwrap().get(&key).cloned() {
        return state;
    }
    let inherited = inherited(c, node, prefixes);
    let state = extend(c, node, inherited);
    prefixes.0.lock().unwrap().insert(key, state.clone());
    state
}

fn inherited(c: &Context<'_>, node: Node<'_>, prefixes: &Prefixes) -> State {
    node.parent().map_or_else(
        || Some(Arc::new(Vec::new())),
        |parent| after(c, parent, prefixes),
    )
}

fn extend(c: &Context<'_>, node: Node<'_>, inherited: State) -> State {
    let mut parts = inherited?;
    if changes(c, node) {
        add_namespace(c, node, Arc::make_mut(&mut parts))?;
    }
    Some(parts)
}

fn changes(c: &Context<'_>, node: Node<'_>) -> bool {
    match node.kind() {
        "class" | "module" | "singleton_class" | "block" | "do_block" => true,
        "assignment" => field(node, "right").is_some_and(|right| new_class_call(c, right)),
        _ => false,
    }
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
            assert_eq!(parent(&c, node), replay(&c, node), "{} {text}", node.kind());
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
