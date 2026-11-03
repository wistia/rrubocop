use super::geometry::{Delta, Element, col};
use crate::correction::Correction;
use crate::parse::source::SourceFile;

pub(super) fn adjustment(
    source: &SourceFile,
    start: usize,
    delta: i64,
    owner: usize,
) -> Option<Option<Correction>> {
    if delta == 0 {
        return Some(None);
    }
    let (left, replacement) = if delta > 0 {
        (start, " ".repeat(delta as usize))
    } else {
        let left = start.checked_sub((-delta) as usize)?;
        if !source.as_bytes()[left..start]
            .iter()
            .all(|b| matches!(b, b' ' | b'\t'))
        {
            return None;
        }
        (left, String::new())
    };
    Some(Some(Correction {
        start: left,
        end: start,
        replacement,
        cop_name: "Layout/HashAlignment",
        cop_index: 0,
        owner: Some(owner),
    }))
}
pub(super) fn edits(
    source: &SourceFile,
    element: Element<'_>,
    delta: Delta,
) -> Option<Vec<Correction>> {
    let owner = element.node.start_byte();
    let mut result = Vec::new();
    let changes = if element.splat() || element.omitted() {
        vec![(owner, delta.key)]
    } else {
        vec![
            (
                element.key.start_byte(),
                delta.key.max(-col(source, element.key)),
            ),
            (element.operator?.start_byte(), delta.separator),
            (element.value?.start_byte(), delta.value),
        ]
    };
    for (start, delta) in changes {
        if let Some(edit) = adjustment(source, start, delta, owner)? {
            result.push(edit);
        }
    }
    Some(result)
}
