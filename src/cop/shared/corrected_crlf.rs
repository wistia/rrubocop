use crate::correction::Correction;
use crate::parse::source::SourceFile;

/// Opt-in parser-buffer normalization, owned by an existing substantive correction.
pub(crate) fn normalize_corrected_crlf(
    source: &SourceFile,
    cop_name: &'static str,
    corrections: &mut Vec<Correction>,
    initial: usize,
) {
    let Some(first) = corrections.get(initial) else {
        return;
    };
    let owner = first.owner;
    let ranges: Vec<_> = corrections[initial..]
        .iter()
        .map(|c| (c.start, c.end))
        .collect();
    for (start, bytes) in source.as_bytes().windows(2).enumerate() {
        if bytes == b"\r\n" && !ranges.iter().any(|(a, b)| start >= *a && start < *b) {
            corrections.push(Correction {
                start,
                end: start + 1,
                replacement: String::new(),
                cop_name,
                cop_index: 0,
                owner,
            });
        }
    }
}
