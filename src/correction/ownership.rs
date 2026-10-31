use super::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::{directives, source::SourceFile};

pub(crate) fn assign_owners(
    source: &SourceFile,
    diagnostics: &[Diagnostic],
    edits: &mut [Correction],
) {
    for edit in edits {
        if edit.owner.is_none() {
            edit.owner = owner(source, diagnostics, edit);
        }
    }
}
fn owner(source: &SourceFile, diagnostics: &[Diagnostic], edit: &Correction) -> Option<usize> {
    let candidates = candidates(source, diagnostics, edit.cop_name);
    if candidates.len() == 1 {
        return Some(candidates[0].0);
    }
    candidates
        .into_iter()
        .filter(|(off, len)| related(edit, *off, *len))
        .min_by_key(|(off, _)| edit.start.abs_diff(*off))
        .map(|(off, _)| off)
}
fn candidates(source: &SourceFile, diagnostics: &[Diagnostic], name: &str) -> Vec<(usize, usize)> {
    diagnostics
        .iter()
        .filter(|d| d.cop_name == name && d.corrected)
        .filter_map(|d| offset(source, d).map(|off| (off, d.highlight_length)))
        .collect()
}

fn related(edit: &Correction, off: usize, len: usize) -> bool {
    (edit.start <= off && off <= edit.end)
        || (off <= edit.start && edit.start < off.saturating_add(len))
}
fn offset(source: &SourceFile, diagnostic: &Diagnostic) -> Option<usize> {
    source.line_col_to_offset(diagnostic.location.line, diagnostic.location.column)
}
pub(crate) fn filter_disabled(source: &SourceFile, edits: &mut Vec<Correction>, ignore: bool) {
    if ignore {
        return;
    }
    let directives = directives::parse(&String::from_utf8_lossy(source.as_bytes()));
    edits.retain(|edit| {
        edit.owner.is_some_and(|owner| {
            !directives.suppresses(edit.cop_name, source.offset_to_line_col(owner).0)
        })
    });
}
