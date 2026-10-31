use super::Correction;
use std::collections::HashMap;

pub(super) struct Accepted {
    pub edit: Correction,
    pub aliases: Vec<Correction>,
}

pub(super) fn select(raw: Vec<Correction>) -> Vec<Accepted> {
    let mut accepted = Vec::new();
    for group in groups(raw) {
        if compatible(&group, &accepted) {
            accept(group, &mut accepted);
        }
    }
    accepted.sort_by_key(|a| order(&a.edit));
    accepted
}
fn groups(mut raw: Vec<Correction>) -> Vec<Vec<Correction>> {
    raw.sort_by_key(order);
    let mut indices = HashMap::new();
    let mut groups: Vec<Vec<Correction>> = Vec::new();
    for edit in raw {
        let index = group_index(&edit, &mut indices, groups.len());
        if index == groups.len() {
            groups.push(Vec::new());
        }
        groups[index].push(edit);
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
fn compatible(group: &[Correction], accepted: &[Accepted]) -> bool {
    group.iter().enumerate().all(|(index, edit)| {
        !group[..index].iter().any(|other| conflicts(edit, other))
            && !accepted.iter().any(|other| conflicts(edit, &other.edit))
    })
}
fn accept(group: Vec<Correction>, accepted: &mut Vec<Accepted>) {
    for edit in group {
        if let Some(duplicate) = accepted.iter_mut().find(|a| identical(&a.edit, &edit)) {
            duplicate.aliases.push(edit);
        } else {
            accepted.push(Accepted {
                edit,
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
