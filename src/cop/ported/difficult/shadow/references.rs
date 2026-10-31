use super::*;
use crate::model::{Read, Write};
mod branches;
mod implicit;
pub(super) use implicit::reads;

pub(super) fn uses(model: &FileModel<'_>, name: &str, n: Node<'_>) -> bool {
    model
        .scopes
        .iter()
        .filter_map(|s| s.entries.get(name))
        .any(|entry| {
            entry
                .reads
                .iter()
                .any(|r| !implicit::is_implicit(model, r.byte) && contains(n, r.byte))
        })
}
pub(super) fn referenced_before(
    model: &FileModel<'_>,
    entry: &Entry,
    reads: &[Read],
    byte: usize,
    config: &CopConfig,
) -> bool {
    reads.iter().any(|r| {
        if implicit::is_implicit(model, r.byte) {
            return config.get_bool("IgnoreImplicitReferences", false) || r.byte <= byte;
        }
        r.byte <= byte && !entry.writes.iter().any(|w| dominates(model, w, r.byte))
    })
}
fn dominates(model: &FileModel<'_>, write: &Write, byte: usize) -> bool {
    let Some(node) = at(model.tree.root_node(), write.byte).map(assignment) else {
        return false;
    };
    if node.end_byte() > byte {
        return false;
    }
    at(model.tree.root_node(), byte).is_some_and(|reference| !branches::exclusive(node, reference))
}
fn contains(node: Node<'_>, byte: usize) -> bool {
    node.start_byte() <= byte && byte < node.end_byte()
}
