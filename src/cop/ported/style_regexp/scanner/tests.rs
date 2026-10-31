use super::captures;

#[test]
fn capture_types_ignore_lookarounds_classes_and_comments() {
    for (pattern, expected) in [
        ("(?<name>a)(b)", (1, 1)),
        ("(?'name'a)(?:b)", (1, 0)),
        ("(?<=a)(b)(?=c)", (0, 1)),
        ("(?<!a)(?>b)(?~c)", (0, 0)),
        ("[()](?<name>a)", (1, 0)),
        ("[a[b]](?<name>a)(b)", (1, 1)),
        ("[]()](?<name>a)(b)", (1, 1)),
        ("[[:alpha:]()](?<name>a)(b)", (1, 1)),
        (r"\((?<name>a)\)", (1, 0)),
        ("(?# (?<ignored> )(a)", (0, 1)),
        ("(?<name>a)(?(<name>)(b)|(c))", (1, 2)),
    ] {
        assert_eq!(captures(pattern, false, &[]), expected, "{pattern}");
    }
}
#[test]
fn extended_modes_follow_their_group_scope() {
    for (pattern, extended, expected) in [
        ("(?<name>a) # (ignored)\n(b)", true, (1, 1)),
        ("(?x:(?<name>a) # (ignored)\n)(b)", false, (1, 1)),
        ("(?x)(?<name>a) # (ignored)\n(b)", false, (1, 1)),
        ("(?-x:(a)#(b))(?<name>c)", true, (1, 2)),
    ] {
        assert_eq!(captures(pattern, extended, &[]), expected, "{pattern}");
    }
}

#[test]
fn capture_only_mode_preserves_counts_without_emitting_tokens() {
    let pattern = r"[a\d](?<name>b)(?x:(c) # ignored (
)(d)";
    let full = super::run(pattern, false, &[], true);
    let counts = super::run(pattern, false, &[], false);
    assert!(!full.tokens.is_empty());
    assert!(counts.tokens.is_empty());
    assert_eq!(counts.captures, full.captures);
}
