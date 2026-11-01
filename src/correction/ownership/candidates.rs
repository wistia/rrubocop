use crate::correction::Correction;
use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

#[derive(Clone, Copy)]
pub(super) struct Candidate {
    pub offset: usize,
    pub end: usize,
    pub order: usize,
}

#[derive(Default)]
struct Sweep {
    active: BTreeSet<(usize, Reverse<usize>)>,
    expires: BinaryHeap<Reverse<(usize, usize, usize)>>,
    next: usize,
}

pub(super) fn assign_candidates(
    candidates: &mut [Candidate],
    mut indices: Vec<usize>,
    edits: &mut [Correction],
) {
    if candidates.len() == 1 {
        for index in indices {
            edits[index].owner = Some(candidates[0].offset);
        }
        return;
    }
    candidates.sort_by_key(|candidate| (candidate.offset, candidate.order));
    indices.sort_by_key(|index| {
        (
            edits[*index].start,
            edits[*index].start != edits[*index].end,
        )
    });
    let mut sweep = Sweep::default();
    for index in indices {
        let edit = &mut edits[index];
        sweep.advance(candidates, edit);
        edit.owner = sweep.nearest(candidates, edit);
    }
}

impl Sweep {
    fn advance(&mut self, candidates: &[Candidate], edit: &Correction) {
        while self.next < candidates.len() && candidates[self.next].offset <= edit.start {
            self.insert(candidates[self.next]);
            self.next += 1;
        }
        while let Some(&Reverse((end, offset, order))) = self.expires.peek() {
            if end > edit.start
                || offset == edit.start
                || (edit.start == edit.end && end == edit.start)
            {
                break;
            }
            self.expires.pop();
            self.active.remove(&(offset, Reverse(order)));
        }
    }

    fn insert(&mut self, candidate: Candidate) {
        self.active
            .insert((candidate.offset, Reverse(candidate.order)));
        self.expires
            .push(Reverse((candidate.end, candidate.offset, candidate.order)));
    }

    fn nearest(&self, candidates: &[Candidate], edit: &Correction) -> Option<usize> {
        let left = self
            .active
            .last()
            .map(|&(offset, Reverse(order))| (offset, order));
        let right = candidates
            .get(candidates.partition_point(|candidate| candidate.offset < edit.start))
            .filter(|candidate| candidate.offset <= edit.end)
            .map(|candidate| (candidate.offset, candidate.order));
        left.into_iter()
            .chain(right)
            .min_by_key(|&(offset, order)| (edit.start.abs_diff(offset), order))
            .map(|(offset, _)| offset)
    }
}
