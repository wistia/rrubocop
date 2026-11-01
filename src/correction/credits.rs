use super::CorrectionSet;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct Credits {
    cops: HashMap<&'static str, CopCredits>,
}

#[derive(Default)]
struct CopCredits {
    owners: HashSet<usize>,
    ranges: Vec<(usize, usize)>,
}

impl Credits {
    pub(super) fn new(set: &CorrectionSet) -> Self {
        let mut credits = Self::default();
        for edit in set.corrections.iter().chain(set.aliases.iter().flatten()) {
            credits.insert(edit);
        }
        for cop in credits.cops.values_mut() {
            cop.merge_ranges();
        }
        credits
    }

    fn insert(&mut self, edit: &super::Correction) {
        let cop = self.cops.entry(edit.cop_name).or_default();
        if let Some(owner) = edit.owner {
            cop.owners.insert(owner);
        } else {
            cop.ranges.push((edit.start, edit.end));
        }
    }

    pub(super) fn contains(&self, name: &str, offset: usize) -> bool {
        self.cops.get(name).is_some_and(|cop| {
            cop.owners.contains(&offset) || {
                let index = cop.ranges.partition_point(|&(start, _)| start <= offset);
                index > 0 && offset <= cop.ranges[index - 1].1
            }
        })
    }
}

impl CopCredits {
    fn merge_ranges(&mut self) {
        self.ranges.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for &(start, end) in &self.ranges {
            if let Some(last) = merged.last_mut().filter(|last| start <= last.1) {
                last.1 = last.1.max(end);
            } else {
                merged.push((start, end));
            }
        }
        self.ranges = merged;
    }
}
