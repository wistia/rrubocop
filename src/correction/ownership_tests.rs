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
