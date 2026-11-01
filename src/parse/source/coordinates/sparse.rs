pub(in crate::parse::source) struct Sparse {
    runs: Vec<(usize, usize)>,
    end: usize,
}

impl Sparse {
    pub(super) fn new(bytes: &[u8]) -> Option<Self> {
        let (run_count, columns) = counts(bytes);
        if run_count > columns / 2 {
            return None;
        }
        Some(Self {
            runs: boundaries(bytes, run_count),
            end: bytes.len(),
        })
    }

    pub(super) fn column_at(&self, offset: usize) -> usize {
        let offset = offset.min(self.end);
        let complete = self.runs.partition_point(|&(end, _)| end <= offset);
        let skipped = self.skipped(complete);
        let partial = self
            .runs
            .get(complete)
            .map_or(0, |&(_, column)| offset.saturating_sub(column + skipped));
        offset - skipped - partial
    }

    pub(super) fn offset_at(&self, column: usize) -> Option<usize> {
        if column == 0 {
            return Some(0);
        }
        if column > self.end - self.skipped(self.runs.len()) {
            return None;
        }
        let complete = self.runs.partition_point(|&(_, at)| at <= column);
        Some(column + self.skipped(complete))
    }

    fn skipped(&self, complete: usize) -> usize {
        complete.checked_sub(1).map_or(0, |index| {
            let (end, column) = self.runs[index];
            end - column
        })
    }
}

fn counts(bytes: &[u8]) -> (usize, usize) {
    let mut runs = 0;
    let mut columns = 0;
    let mut continuation = false;
    for byte in bytes {
        let next = byte & 0xC0 == 0x80;
        runs += usize::from(next && !continuation);
        columns += usize::from(!next);
        continuation = next;
    }
    (runs, columns)
}

fn boundaries(bytes: &[u8], capacity: usize) -> Vec<(usize, usize)> {
    let mut runs = Vec::with_capacity(capacity);
    let mut columns = 0;
    let mut continuation = false;
    for (offset, byte) in bytes.iter().enumerate() {
        let next = byte & 0xC0 == 0x80;
        if continuation && !next {
            runs.push((offset, columns));
        }
        columns += usize::from(!next);
        continuation = next;
    }
    if continuation {
        runs.push((bytes.len(), columns));
    }
    runs
}
