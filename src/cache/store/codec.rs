use crate::diagnostic::Diagnostic;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize)]
struct Encoded<'a> {
    format: u8,
    ts: u64,
    diagnostics: Vec<Diagnostic>,
    source_lines: Vec<&'a str>,
    line_indices: Vec<usize>,
}

#[derive(Serialize)]
struct Empty {
    ts: u64,
    diagnostics: [Diagnostic; 0],
}

#[derive(Deserialize)]
struct Decoded {
    #[serde(rename = "ts")]
    _ts: u64,
    #[serde(default)]
    format: Option<u8>,
    diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    source_lines: Option<Vec<String>>,
    #[serde(default)]
    line_indices: Option<Vec<usize>>,
}

pub(super) fn encode(ts: u64, diagnostics: &[Diagnostic]) -> serde_json::Result<Vec<u8>> {
    if diagnostics.is_empty() {
        return serde_json::to_vec(&Empty {
            ts,
            diagnostics: [],
        });
    }
    let mut encoded = Encoded {
        format: 2,
        ts,
        diagnostics: Vec::new(),
        source_lines: Vec::new(),
        line_indices: Vec::new(),
    };
    let mut identities = HashMap::new();
    let empty = Arc::<str>::default();
    for diagnostic in diagnostics {
        encoded.push(diagnostic, &mut identities, &empty);
    }
    serde_json::to_vec(&encoded)
}

impl<'a> Encoded<'a> {
    fn push(
        &mut self,
        diagnostic: &'a Diagnostic,
        identities: &mut HashMap<(usize, usize), usize>,
        empty: &Arc<str>,
    ) {
        let identity = (
            diagnostic.source_line.as_ptr() as usize,
            diagnostic.source_line.len(),
        );
        let index = *identities.entry(identity).or_insert_with(|| {
            self.source_lines.push(&diagnostic.source_line);
            self.source_lines.len() - 1
        });
        self.line_indices.push(index);
        let mut metadata = diagnostic.clone();
        metadata.source_line = empty.clone();
        self.diagnostics.push(metadata);
    }
}

pub(super) fn decode(bytes: &[u8]) -> Option<Vec<Diagnostic>> {
    let decoded: Decoded = serde_json::from_slice(bytes).ok()?;
    decoded.restore()
}

impl Decoded {
    fn restore(mut self) -> Option<Vec<Diagnostic>> {
        match (self.format, self.source_lines, self.line_indices) {
            (None, None, None) => {}
            (Some(2), Some(lines), Some(indices)) => {
                restore_lines(&mut self.diagnostics, lines, indices)?
            }
            _ => return None,
        }
        Some(self.diagnostics)
    }
}

fn restore_lines(
    diagnostics: &mut [Diagnostic],
    lines: Vec<String>,
    indices: Vec<usize>,
) -> Option<()> {
    if indices.len() != diagnostics.len() {
        return None;
    }
    let lines: Vec<Arc<str>> = lines.into_iter().map(Arc::from).collect();
    for (diagnostic, index) in diagnostics.iter_mut().zip(indices) {
        diagnostic.source_line = lines.get(index)?.clone();
    }
    Some(())
}

#[derive(Deserialize)]
struct Timestamp {
    ts: u64,
}

pub(super) fn timestamp(bytes: &[u8]) -> Option<u64> {
    serde_json::from_slice::<Timestamp>(bytes)
        .ok()
        .map(|value| value.ts)
}

#[cfg(test)]
mod tests;
