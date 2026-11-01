use super::*;

fn old_select(raw: Vec<Correction>) -> Vec<Accepted> {
    let mut accepted: Vec<Accepted> = Vec::new();
    for group in old_groups(raw) {
        if old_compatible(&group, &accepted) {
            old_accept(group, &mut accepted);
        }
    }
    accepted.sort_by_key(|a| order(&a.edit));
    accepted
}

fn old_groups(mut raw: Vec<Correction>) -> Vec<Vec<Correction>> {
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

fn old_compatible(group: &[Correction], accepted: &[Accepted]) -> bool {
    group.iter().enumerate().all(|(index, edit)| {
        !group[..index].iter().any(|other| conflicts(edit, other))
            && !accepted.iter().any(|other| conflicts(edit, &other.edit))
    })
}

fn old_accept(group: Vec<Correction>, accepted: &mut Vec<Accepted>) {
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

fn signature(accepted: Vec<Accepted>) -> Vec<String> {
    accepted
        .into_iter()
        .map(|a| format!("{:?}:{:?}", a.edit, a.aliases))
        .collect()
}

fn edit(index: usize) -> Correction {
    let start = index % 4;
    Correction {
        start,
        end: start + (index / 4) % 3,
        replacement: if (index / 12).is_multiple_of(2) {
            "x"
        } else {
            "y"
        }
        .into(),
        cop_name: if (index / 24).is_multiple_of(2) {
            "Test/A"
        } else {
            "Test/B"
        },
        cop_index: index / 24 % 2,
        owner: match index / 48 % 3 {
            0 => None,
            1 => Some(0),
            _ => Some(1),
        },
    }
}

#[test]
fn indexed_selection_matches_old_selection_across_boundaries_aliases_and_groups() {
    for first in 0..144 {
        for second in 0..144 {
            let edits = vec![
                edit(first),
                edit(second),
                edit((first * 7 + second * 11) % 144),
                edit((first * 13 + second * 3) % 144),
            ];
            assert_eq!(
                signature(select(edits.clone())),
                signature(old_select(edits)),
                "{first},{second}"
            );
        }
    }
}

#[test]
fn indexed_selection_matches_longer_mixed_owner_groups() {
    let mut seed = 37usize;
    for _ in 0..2000 {
        let edits: Vec<_> = (0..16)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                edit(seed % 144)
            })
            .collect();
        assert_eq!(
            signature(select(edits.clone())),
            signature(old_select(edits))
        );
    }
}
