use super::tests::{diag, edit};
use super::*;
use crate::parse::source::SourceFile;

fn owned(
    start: usize,
    end: usize,
    replacement: &str,
    name: &'static str,
    owner: usize,
) -> Correction {
    Correction {
        owner: Some(owner),
        ..edit(start, end, replacement, name)
    }
}
#[test]
fn edits_are_credited_to_the_offense_not_their_edited_ranges() {
    let source = SourceFile::from_bytes("a.rb", b"receiver.method(argument)\n".to_vec());
    let set = CorrectionSet::from_vec(vec![owned(9, 15, "other", "Test/Call", 0)]);
    let mut diagnostics = vec![diag("Test/Call", 0)];
    reconcile_corrected(&mut diagnostics, &source, &set);
    assert!(diagnostics[0].corrected);
}
#[test]
fn a_conflict_rejects_every_edit_owned_by_that_offense() {
    let source = SourceFile::from_bytes("a.rb", b"abcdef\n".to_vec());
    let mut diagnostics = vec![diag("Test/First", 0), diag("Test/Second", 2)];
    let set = CorrectionSet::from_vec(vec![
        owned(0, 2, "XX", "Test/First", 0),
        owned(1, 3, "YY", "Test/Second", 2),
        owned(4, 5, "Z", "Test/Second", 2),
    ]);
    assert_eq!(set.apply(source.as_bytes()), b"XXcdef\n");
    reconcile_corrected(&mut diagnostics, &source, &set);
    assert!(diagnostics[0].corrected);
    assert!(!diagnostics[1].corrected);
}
#[test]
fn identical_edits_keep_their_different_offense_owners() {
    let source = SourceFile::from_bytes("a.rb", b"abcdef\n".to_vec());
    let set = CorrectionSet::from_vec(vec![
        owned(3, 5, "X", "Test/First", 0),
        owned(3, 5, "X", "Test/Second", 1),
        owned(5, 6, "Y", "Test/Second", 1),
    ]);
    let mut diagnostics = vec![diag("Test/First", 0), diag("Test/Second", 1)];
    assert_eq!(set.apply(source.as_bytes()), b"abcXY\n");
    reconcile_corrected(&mut diagnostics, &source, &set);
    assert!(diagnostics.iter().all(|d| d.corrected));
}
#[test]
fn insertion_boundaries_do_not_duplicate_the_replaced_source() {
    let set = CorrectionSet::from_vec(vec![
        owned(0, 2, "X", "Test/Replacement", 0),
        owned(0, 0, "before", "Test/Insertion", 3),
        owned(2, 2, "after", "Test/Insertion", 3),
    ]);
    assert_eq!(set.apply(b"ab"), b"beforeXafter");
}

#[test]
fn independent_insertions_at_the_same_position_are_serialized() {
    let source = SourceFile::from_bytes("a.rb", b"{}\n".to_vec());
    let set = CorrectionSet::from_vec(vec![
        owned(1, 1, "a: 1", "Test/First", 0),
        owned(1, 1, "b: 2", "Test/Second", 0),
    ]);
    assert_eq!(set.apply(source.as_bytes()), b"{a: 1}\n");
    let mut diagnostics = vec![diag("Test/First", 0), diag("Test/Second", 0)];
    reconcile_corrected(&mut diagnostics, &source, &set);
    assert!(diagnostics[0].corrected);
    assert!(!diagnostics[1].corrected);
}
#[test]
fn same_owner_insertions_keep_their_intentional_order() {
    let set = CorrectionSet::from_vec(vec![
        owned(1, 1, "first", "Test/One", 0),
        owned(1, 1, "second", "Test/One", 0),
    ]);
    assert_eq!(set.apply(b"{}"), b"{firstsecond}");
}

#[test]
fn indexed_ownership_preserves_nearest_match_original_order_and_explicit_owners() {
    let source = SourceFile::from_bytes("a.rb", vec![b'x'; 80]);
    for first in 0..20 {
        for second in 0..20 {
            for third in 0..20 {
                let diagnostics = owner_diagnostics([first, second, third]);
                let mut edits = owner_edits();
                let expected: Vec<_> = edits
                    .iter()
                    .map(|edit| old_owner_with_insertion_endpoint(&source, &diagnostics, edit))
                    .collect();
                assign_owners(&source, &diagnostics, &mut edits);
                assert_eq!(
                    edits.iter().map(|edit| edit.owner).collect::<Vec<_>>(),
                    expected,
                    "{first},{second},{third}"
                );
            }
        }
    }
}

fn owner_diagnostics(values: [usize; 3]) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<_> = values
        .into_iter()
        .map(|value| {
            let mut diagnostic = diag("Test/A", value % 5);
            diagnostic.highlight_length = value / 5;
            diagnostic
        })
        .collect();
    diagnostics.push(diag("Test/B", 1));
    diagnostics
}

fn owner_edits() -> Vec<Correction> {
    let mut edits: Vec<_> = (0..8)
        .flat_map(|start| (start..8).map(move |end| edit(start, end, "X", "Test/A")))
        .collect();
    edits.push(owned(0, 1, "X", "Test/A", 77));
    edits
}

fn old_owner_with_insertion_endpoint(
    source: &SourceFile,
    diagnostics: &[Diagnostic],
    edit: &Correction,
) -> Option<usize> {
    if edit.owner.is_some() {
        return edit.owner;
    }
    let candidates: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.cop_name == edit.cop_name && d.corrected)
        .filter_map(|d| {
            source
                .line_col_to_offset(d.location.line, d.location.column)
                .map(|off| (off, d.highlight_length))
        })
        .collect();
    if candidates.len() == 1 {
        return Some(candidates[0].0);
    }
    candidates
        .into_iter()
        .filter(|&(off, len)| old_related(edit, off, len))
        .min_by_key(|&(off, _)| edit.start.abs_diff(off))
        .map(|(off, _)| off)
}

fn old_related(edit: &Correction, off: usize, len: usize) -> bool {
    (edit.start <= off && off <= edit.end)
        || (off <= edit.start && edit.start < off.saturating_add(len))
        || (edit.start == edit.end && off <= edit.start && edit.start == off.saturating_add(len))
}

#[test]
fn singleton_owner_fallback_remains_independent_of_edit_range() {
    let source = SourceFile::from_bytes("a.rb", vec![b'x'; 80]);
    let mut edits = vec![edit(50, 51, "X", "Test/A"), edit(0, 0, "X", "Test/B")];
    assign_owners(&source, &[diag("Test/A", 1)], &mut edits);
    assert_eq!(edits[0].owner, Some(1));
    assert_eq!(edits[1].owner, None);
}

#[test]
fn indexed_credits_match_original_scan_including_aliases_and_inclusive_boundaries() {
    for first in 0..6 {
        for second in 0..6 {
            let set = CorrectionSet::from_vec(vec![
                owned(first, first + 1, "X", "Test/A", second),
                owned(first, first + 1, "X", "Test/B", second + 1),
                edit(second, second + 2, "Y", "Test/A"),
                edit(7, 7, "Z", "Test/B"),
            ]);
            let credits = credits::Credits::new(&set);
            for cop in ["Test/A", "Test/B", "Test/C"] {
                for offset in 0..10 {
                    let expected = set
                        .corrections
                        .iter()
                        .chain(set.aliases.iter().flatten())
                        .any(|edit| {
                            edit.cop_name == cop
                                && edit.owner.map_or_else(
                                    || edit.start <= offset && offset <= edit.end,
                                    |owner| owner == offset,
                                )
                        });
                    assert_eq!(credits.contains(cop, offset), expected);
                }
            }
        }
    }
}
