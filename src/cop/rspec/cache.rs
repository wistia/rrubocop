use crate::cop::CopConfig;
use crate::parse::source::SourceFile;
use std::collections::HashMap;
use std::sync::Mutex;
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct NodeRange {
    pub start: usize,
    pub end: usize,
    kind: u16,
}
impl NodeRange {
    pub fn new(node: Node<'_>) -> Self {
        Self {
            start: node.start_byte(),
            end: node.end_byte(),
            kind: node.kind_id(),
        }
    }
    pub fn resolve(self, root: Node<'_>) -> Option<Node<'_>> {
        let mut node = root.named_descendant_for_byte_range(self.start, self.end)?;
        loop {
            if Self::new(node) == self {
                return Some(node);
            }
            let parent = node.parent()?;
            if parent.start_byte() < self.start || parent.end_byte() > self.end {
                return None;
            }
            node = parent;
        }
    }
}
struct NodeEntries<T>(Mutex<HashMap<NodeRange, T>>);

// Each fact uses its own wrapper type: the cache key is the fact type and source range.
pub(super) fn node_fact<T: Clone + Send + Sync + 'static>(
    source: &SourceFile,
    node: Node<'_>,
    build: impl FnOnce() -> T,
) -> T {
    let entries = source.cached(0, || NodeEntries::<T>(Mutex::new(HashMap::new())));
    let key = NodeRange::new(node);
    if let Some(value) = entries.0.lock().unwrap().get(&key) {
        return value.clone();
    }
    let value = build();
    entries.0.lock().unwrap().insert(key, value.clone());
    value
}

type Configuration = Vec<(String, Option<serde_yml::Value>)>;
struct ConfigEntries<T>(Mutex<Vec<(Configuration, T)>>);
pub(crate) fn config_fact<T: Clone + Send + Sync + 'static>(
    source: &SourceFile,
    config: &CopConfig,
    keys: &[&str],
    build: impl FnOnce() -> T,
) -> T {
    let entries = source.cached(0, || ConfigEntries::<T>(Mutex::new(Vec::new())));
    if let Some((_, value)) = entries.0.lock().unwrap().iter().find(|(snapshot, _)| {
        snapshot.len() == keys.len()
            && snapshot.iter().zip(keys).all(|((key, value), wanted)| {
                key == wanted && value.as_ref() == config.options.get(*wanted)
            })
    }) {
        return value.clone();
    }
    let value = build();
    let snapshot = keys
        .iter()
        .map(|key| ((*key).to_owned(), config.options.get(*key).cloned()))
        .collect();
    entries.0.lock().unwrap().push((snapshot, value.clone()));
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[derive(Clone, Debug, PartialEq)]
    struct Prepared(usize);
    #[test]
    fn configuration_snapshots_reuse_failures_and_keep_nested_overrides_distinct() {
        let source = SourceFile::from_bytes("spec/a_spec.rb", vec![]);
        let count = Cell::new(0);
        let prepare = || {
            count.set(count.get() + 1);
            Prepared(count.get())
        };
        let mut config = CopConfig::default();
        assert_eq!(
            config_fact(&source, &config, &["Patterns"], prepare),
            Prepared(1)
        );
        config
            .options
            .insert("Other".into(), serde_yml::Value::Bool(true));
        assert_eq!(
            config_fact(&source, &config, &["Patterns"], prepare),
            Prepared(1)
        );
        config
            .options
            .insert("Patterns".into(), serde_yml::from_str("['[']").unwrap());
        assert_eq!(
            config_fact(&source, &config, &["Patterns"], prepare),
            Prepared(2)
        );
        config.options.remove("Patterns");
        assert_eq!(
            config_fact(&source, &config, &["Patterns"], prepare),
            Prepared(1)
        );
        let other = SourceFile::from_bytes("spec/b_spec.rb", vec![]);
        assert_eq!(
            config_fact(&other, &config, &["Patterns"], prepare),
            Prepared(3)
        );
    }

    #[test]
    fn owned_node_ranges_resolve_against_reparsed_trees_without_retaining_nodes() {
        let source = SourceFile::from_bytes(
            "spec/a_spec.rb",
            b"describe A do; it('x') { work }; end".to_vec(),
        );
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let call = tree.root_node().named_child(0).unwrap();
        let count = Cell::new(0);
        let prepare = || {
            count.set(count.get() + 1);
            Prepared(count.get())
        };
        assert_eq!(node_fact(&source, call, prepare), Prepared(1));
        let range = NodeRange::new(call);
        let second = crate::parse::parse_ruby(&source).unwrap();
        let resolved = range.resolve(second.root_node()).unwrap();
        assert_eq!(resolved.kind(), "call");
        assert_eq!(node_fact(&source, resolved, prepare), Prepared(1));
    }
}
