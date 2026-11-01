use super::{Correction, conflicts, identical, same_owner};
use std::collections::BTreeMap;
use std::ops::Bound::Excluded;

#[derive(Default)]
pub(super) struct ConflictIndex<'a> {
    ranges: BTreeMap<usize, &'a Correction>,
    insertions: BTreeMap<usize, Insertions<'a>>,
}

struct Insertions<'a> {
    first: &'a Correction,
    multiple: bool,
}

impl<'a> ConflictIndex<'a> {
    pub(super) fn conflicts(&self, edit: &Correction) -> bool {
        if self
            .ranges
            .range(..=edit.start)
            .next_back()
            .is_some_and(|(_, other)| conflicts(edit, other))
        {
            return true;
        }
        if edit.start == edit.end {
            return self
                .insertions
                .get(&edit.start)
                .is_some_and(|others| others.conflicts(edit));
        }
        self.interior_conflicts(edit)
    }

    fn interior_conflicts(&self, edit: &Correction) -> bool {
        self.ranges
            .range((Excluded(edit.start), Excluded(edit.end)))
            .next()
            .is_some()
            || self
                .insertions
                .range((Excluded(edit.start), Excluded(edit.end)))
                .next()
                .is_some()
    }

    pub(super) fn insert(&mut self, edit: &'a Correction) {
        if edit.start == edit.end {
            self.insertions
                .entry(edit.start)
                .and_modify(|others| others.multiple |= !identical(others.first, edit))
                .or_insert_with(|| Insertions {
                    first: edit,
                    multiple: false,
                });
        } else {
            self.ranges.entry(edit.start).or_insert(edit);
        }
    }
}

impl Insertions<'_> {
    fn conflicts(&self, edit: &Correction) -> bool {
        if same_owner(self.first, edit) {
            return false;
        }
        self.multiple || !identical(self.first, edit)
    }
}
