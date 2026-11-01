use super::Correction;
use std::collections::HashMap;
mod index;
use index::ConflictIndex;

pub(super) struct Accepted {
    pub edit: Correction,
    pub aliases: Vec<Correction>,
}

struct Selected {
    edit: usize,
    aliases: Vec<usize>,
}

pub(super) fn select(mut raw: Vec<Correction>) -> Vec<Accepted> {
    raw.sort_by_key(order);
    let selected = select_indices(&raw);
    materialize(raw, selected)
}

fn materialize(raw: Vec<Correction>, selected: Vec<Selected>) -> Vec<Accepted> {
    let mut edits: Vec<_> = raw.into_iter().map(Some).collect();
    let mut accepted: Vec<_> = selected
        .into_iter()
        .map(|selected| Accepted {
            edit: edits[selected.edit].take().unwrap(),
            aliases: selected
                .aliases
                .into_iter()
                .map(|index| edits[index].take().unwrap())
                .collect(),
        })
        .collect();
    accepted.sort_by_key(|a| order(&a.edit));
    accepted
}
fn select_indices(raw: &[Correction]) -> Vec<Selected> {
    let mut selected = Vec::new();
    let mut conflicts = ConflictIndex::default();
    let mut duplicates = HashMap::new();
    for group in groups(raw) {
        if compatible(&group, raw, &conflicts) {
            accept(group, raw, &mut selected, &mut conflicts, &mut duplicates);
        }
    }
    selected
}
fn groups(raw: &[Correction]) -> Vec<Vec<usize>> {
    let mut indices = HashMap::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (index, edit) in raw.iter().enumerate() {
        let group = group_index(edit, &mut indices, groups.len());
        if group == groups.len() {
            groups.push(Vec::new());
        }
        groups[group].push(index);
    }
    groups
}
fn group_index(
    edit: &Correction,
    indices: &mut HashMap<(&'static str, usize), usize>,
    next: usize,
) -> usize {
    edit.owner.map_or(next, |owner| {
        *indices.entry((edit.cop_name, owner)).or_insert(next)
    })
}
fn compatible<'a>(group: &[usize], raw: &'a [Correction], accepted: &ConflictIndex<'a>) -> bool {
    let mut within_group = ConflictIndex::default();
    for &index in group {
        let edit = &raw[index];
        if accepted.conflicts(edit) || within_group.conflicts(edit) {
            return false;
        }
        within_group.insert(edit);
    }
    true
}
fn accept<'a>(
    group: Vec<usize>,
    raw: &'a [Correction],
    selected: &mut Vec<Selected>,
    conflicts: &mut ConflictIndex<'a>,
    duplicates: &mut HashMap<(usize, usize, &'a str), usize>,
) {
    for index in group {
        let edit = &raw[index];
        let key = (edit.start, edit.end, edit.replacement.as_str());
        if let Some(&accepted) = duplicates.get(&key) {
            selected[accepted].aliases.push(index);
        } else {
            duplicates.insert(key, selected.len());
            conflicts.insert(edit);
            selected.push(Selected {
                edit: index,
                aliases: Vec::new(),
            });
        }
    }
}
fn identical(a: &Correction, b: &Correction) -> bool {
    a.start == b.start && a.end == b.end && a.replacement == b.replacement
}
fn conflicts(a: &Correction, b: &Correction) -> bool {
    if identical(a, b) {
        return false;
    }
    if a.start == a.end {
        return insertion_conflicts(a, b);
    }
    if b.start == b.end {
        return insertion_conflicts(b, a);
    }
    a.start < b.end && b.start < a.end
}
fn insertion_conflicts(insertion: &Correction, other: &Correction) -> bool {
    if other.start == other.end {
        return insertion.start == other.start && !same_owner(insertion, other);
    }
    other.start < insertion.start && insertion.start < other.end
}

fn same_owner(a: &Correction, b: &Correction) -> bool {
    a.owner.is_some() && a.owner == b.owner && a.cop_name == b.cop_name
}

fn order(edit: &Correction) -> (usize, bool, usize) {
    (edit.start, edit.start != edit.end, edit.cop_index)
}

#[cfg(test)]
mod tests;
