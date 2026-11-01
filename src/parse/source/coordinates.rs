use super::SourceFile;
use std::sync::Arc;

pub(super) enum Columns {
    Ascii(usize),
    Unicode { starts: Vec<usize>, end: usize },
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
            let bytes = &self.content[start..end];
            Columns::new(bytes)
        })
    }
}

impl Columns {
    fn new(bytes: &[u8]) -> Self {
        if bytes.is_ascii() {
            Self::Ascii(bytes.len())
        } else {
            Self::Unicode {
                starts: bytes
                    .iter()
                    .enumerate()
                    .filter_map(|(index, byte)| (byte & 0xC0 != 0x80).then_some(index))
                    .collect(),
                end: bytes.len(),
            }
        }
    }

    pub(super) fn column_at(&self, offset: usize) -> usize {
        match self {
            Self::Ascii(_) => offset,
            Self::Unicode { starts, .. } => starts.partition_point(|&start| start < offset),
        }
    }

    pub(super) fn offset_at(&self, column: usize) -> Option<usize> {
        match self {
            Self::Ascii(end) => (column <= *end).then_some(column),
            Self::Unicode { starts, end } => {
                if column == 0 {
                    return Some(0);
                }
                starts
                    .get(column)
                    .copied()
                    .or_else(|| (column == starts.len()).then_some(*end))
            }
        }
    }
}
