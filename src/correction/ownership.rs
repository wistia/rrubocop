use super::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::{directives, source::SourceFile};
use std::collections::HashMap;
mod candidates;
use candidates::{Candidate, assign_candidates};

pub(crate) fn assign_owners(
    source: &SourceFile,
    diagnostics: &[Diagnostic],
    edits: &mut [Correction],
) {
    if edits.iter().all(|edit| edit.owner.is_some()) {
        return;
    }
    let mut candidates = diagnostic_candidates(source, diagnostics);
    let mut by_cop: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, edit) in edits
        .iter()
        .enumerate()
        .filter(|(_, edit)| edit.owner.is_none())
    {
        by_cop.entry(edit.cop_name).or_default().push(index);
    }
    for (cop, indices) in by_cop {
        if let Some(candidates) = candidates.get_mut(cop) {
            assign_candidates(candidates, indices, edits);
        }
    }
}

fn diagnostic_candidates<'a>(
    source: &SourceFile,
    diagnostics: &'a [Diagnostic],
) -> HashMap<&'a str, Vec<Candidate>> {
    let mut candidates: HashMap<&str, Vec<Candidate>> = HashMap::new();
    for (order, diagnostic) in diagnostics.iter().enumerate().filter(|(_, d)| d.corrected) {
        if let Some(offset) =
            source.line_col_to_offset(diagnostic.location.line, diagnostic.location.column)
        {
            candidates
                .entry(&diagnostic.cop_name)
                .or_default()
                .push(Candidate {
                    offset,
                    end: offset.saturating_add(diagnostic.highlight_length),
                    order,
                });
        }
    }
    candidates
}

pub(crate) fn filter_disabled(source: &SourceFile, edits: &mut Vec<Correction>, ignore: bool) {
    if ignore {
        return;
    }
    let directives = directives::cached(source);
    edits.retain(|edit| {
        edit.owner.is_some_and(|owner| {
            !directives.suppresses(edit.cop_name, source.offset_to_line_col(owner).0)
        })
    });
}
