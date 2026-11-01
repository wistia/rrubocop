use super::SourceFile;
use std::sync::Arc;

mod sparse;
use sparse::Sparse;

pub(super) enum Columns {
    Ascii(usize),
    Dense { starts: Vec<usize>, end: usize },
    Sparse(Sparse),
}

impl SourceFile {
    pub(super) fn columns(&self, line: usize) -> Arc<Columns> {
        self.cached(line, || {
            let start = self.line_start(line).unwrap();
            let end = self
                .line_starts
                .get(line)
                .copied()
                .unwrap_or(self.content.len());
            Columns::new(&self.content[start..end])
        })
    }
}

impl Columns {
    fn new(bytes: &[u8]) -> Self {
        if bytes.is_ascii() {
            return Self::Ascii(bytes.len());
        }
        if let Some(sparse) = Sparse::new(bytes) {
            return Self::Sparse(sparse);
        }
        Self::Dense {
            starts: bytes
                .iter()
                .enumerate()
                .filter_map(|(index, byte)| (byte & 0xC0 != 0x80).then_some(index))
                .collect(),
            end: bytes.len(),
        }
    }

    pub(super) fn column_at(&self, offset: usize) -> usize {
        match self {
            Self::Ascii(_) => offset,
            Self::Dense { starts, .. } => starts.partition_point(|&start| start < offset),
            Self::Sparse(sparse) => sparse.column_at(offset),
        }
    }

    pub(super) fn offset_at(&self, column: usize) -> Option<usize> {
        match self {
            Self::Ascii(end) => (column <= *end).then_some(column),
            Self::Dense { starts, end } => {
                if column == 0 {
                    return Some(0);
                }
                starts
                    .get(column)
                    .copied()
                    .or_else(|| (column == starts.len()).then_some(*end))
            }
            Self::Sparse(sparse) => sparse.offset_at(column),
        }
    }
}

#[cfg(test)]
mod tests;
