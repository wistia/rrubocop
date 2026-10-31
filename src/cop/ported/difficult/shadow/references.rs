use super::*;
use crate::model::{Read, Write};
mod branches;
mod implicit;
pub(super) use implicit::reads;

pub(super) use implicit::Index;
pub(super) fn uses(index: &Index, name: &str, n: Node<'_>) -> bool {
    index.explicit.get(name).is_some_and(|bytes| {
        let start = bytes.partition_point(|byte| *byte < n.start_byte());
        bytes.get(start).is_some_and(|byte| *byte < n.end_byte())
    })
}
pub(super) fn referenced_before(
    model: &FileModel<'_>,
    index: &Index,
    entry: &Entry,
    reads: &[Read],
    byte: usize,
    config: &CopConfig,
) -> bool {
    reads.iter().any(|r| {
        if index.implicit.contains(&r.byte) {
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
