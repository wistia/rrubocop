use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::diagnostic::Location;

mod cache;
use cache::AnalysisCache;

#[derive(Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: Vec<u8>,
    line_starts: Vec<usize>,
    analysis: AnalysisCache,
}

impl SourceFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let content =
            std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        Ok(Self::from_bytes(path, content))
    }

    pub fn from_bytes(path: impl Into<PathBuf>, content: Vec<u8>) -> Self {
        let content = truncate_at_end_marker(content);
        Self {
            path: path.into(),
            line_starts: compute_line_starts(&content),
            content,
            analysis: AnalysisCache::default(),
        }
    }

    pub(crate) fn cached<T: Send + Sync + 'static>(
        &self,
        key: usize,
        initialize: impl FnOnce() -> T,
    ) -> std::sync::Arc<T> {
        self.analysis.get(key, initialize)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.content
    }

    pub fn path_str(&self) -> &str {
        self.path.to_str().unwrap_or("")
    }

    pub fn lines(&self) -> impl Iterator<Item = &[u8]> {
        self.content.split(|&b| b == b'\n')
    }

    /// Number of lines (including a trailing empty line after a final `\n`).
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn offset_to_line_col(&self, byte_offset: usize) -> (usize, usize) {
        let line_idx = match self.line_starts.binary_search(&byte_offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line_bytes = &self.content[self.line_starts[line_idx]..byte_offset];
        (line_idx + 1, utf8_byte_index_to_column(line_bytes))
    }

    pub fn line_start(&self, line: usize) -> Option<usize> {
        if line == 0 || line > self.line_starts.len() {
            None
        } else {
            Some(self.line_starts[line - 1])
        }
    }

    /// Line content without trailing `\n` / `\r` (1-based line).
    pub fn line_text(&self, line: usize) -> Option<&str> {
        let start = self.line_start(line)?;
        std::str::from_utf8(trim_line_ending(
            &self.content[start
                ..self
                    .line_starts
                    .get(line)
                    .copied()
                    .unwrap_or(self.content.len())],
        ))
        .ok()
    }

    /// Byte offset of (1-based line, 0-based display column). Best-effort for ASCII.
    pub fn line_col_to_offset(&self, line: usize, column: usize) -> Option<usize> {
        let start = self.line_start(line)?;
        Some(start + column)
    }

    pub fn location_at(&self, byte_offset: usize) -> Location {
        let (line, column) = self.offset_to_line_col(byte_offset);
        Location { line, column }
    }
}

/// UTF-8 prefix length → 0-based display column (matches `offset_to_line_col`).
pub fn utf8_byte_index_to_column(prefix: &[u8]) -> usize {
    prefix.iter().filter(|&&b| (b & 0xC0) != 0x80).count()
}

/// UTF-8 byte index within `s` → 0-based display column.
pub fn byte_index_to_column(s: &str, byte_idx: usize) -> usize {
    utf8_byte_index_to_column(s.as_bytes().get(..byte_idx.min(s.len())).unwrap_or(&[]))
}

fn trim_line_ending(mut slice: &[u8]) -> &[u8] {
    if slice.last() == Some(&b'\n') {
        slice = &slice[..slice.len() - 1];
    }
    if slice.last() == Some(&b'\r') {
        slice = &slice[..slice.len() - 1];
    }
    slice
}

/// RuboCop ignores source after a lone `__END__` line (DATA section).
fn truncate_at_end_marker(content: Vec<u8>) -> Vec<u8> {
    let mut line_start = 0usize;
    for (i, &b) in content.iter().enumerate() {
        if b == b'\n' {
            if &content[line_start..i] == b"__END__"
                || content[line_start..i].strip_suffix(b"\r") == Some(b"__END__")
            {
                return content[..=i].to_vec();
            }
            line_start = i + 1;
        }
    }
    if &content[line_start..] == b"__END__"
        || content[line_start..].strip_suffix(b"\r") == Some(b"__END__")
    {
        return content[..line_start.saturating_add(7)].to_vec();
    }
    content
}

fn compute_line_starts(content: &[u8]) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, &b) in content.iter().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}
