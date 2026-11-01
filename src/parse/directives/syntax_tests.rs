use super::*;
use crate::parse::{parse_ruby, source::SourceFile};
use std::sync::Arc;

#[test]
fn literal_contents_and_block_comments_do_not_disable_cops() {
    for source in [
        "value = <<~TEXT\n# rubocop:disable Layout/TrailingWhitespace\nTEXT\nx = 1  \n",
        "value = \"first\n# rubocop:disable Layout/TrailingWhitespace\nlast\"\nx = 1  \n",
        "value = %q(first\n# rubocop:disable Layout/TrailingWhitespace\nlast)\nx = 1  \n",
        "value = %r{first\n# rubocop:disable Layout/TrailingWhitespace\nlast}\nx = 1  \n",
        "=begin\n# rubocop:disable Layout/TrailingWhitespace\n=end\nx = 1  \n",
    ] {
        let directives = parse(source);
        assert!(
            !directives.suppresses("Layout/TrailingWhitespace", 4),
            "{source}"
        );
        assert!(directives.comment_lines().is_empty(), "{source}");
    }
}

#[test]
fn real_comments_inside_interpolation_and_after_literals_still_apply() {
    for source in [
        "value = \"#{\n# rubocop:disable Layout/TrailingWhitespace\nx = 1  \n}\"\n",
        "value = <<~TEXT\n#{\n# rubocop:disable Layout/TrailingWhitespace\nx = 1  \n}\nTEXT\n",
        "value = %q(# fake) # rubocop:disable Layout/TrailingWhitespace\n",
    ] {
        let directives = parse(source);
        let line = if source.starts_with("value = %q") {
            1
        } else if source.contains("<<~") {
            4
        } else {
            3
        };
        assert!(
            directives.suppresses("Layout/TrailingWhitespace", line),
            "{source}"
        );
    }
}

#[test]
fn directives_reuse_the_existing_tree_and_typed_source_cache() {
    let source = SourceFile::from_bytes(
        "a.rb",
        b"# rubocop:disable Layout/TrailingWhitespace\nx = 1  \n".to_vec(),
    );
    let tree = parse_ruby(&source).unwrap();
    let first = for_source(&source, &tree);
    assert!(Arc::ptr_eq(&first, &cached(&source)));
    assert!(Arc::ptr_eq(&first, &for_source(&source, &tree)));
    assert!(first.suppresses("Layout/TrailingWhitespace", 2));
    let corrected = SourceFile::from_bytes("a.rb", b"x = 1\n".to_vec());
    assert!(!cached(&corrected).suppresses("Layout/TrailingWhitespace", 2));
}

#[test]
fn selectors_share_the_same_per_cop_ranges() {
    for (disable, enable) in [
        ("Layout/SpaceAfterComma", "all"),
        ("Layout/SpaceAfterComma", "Layout"),
        ("all", "Layout/SpaceAfterComma"),
        ("Layout", "Layout/SpaceAfterComma"),
    ] {
        let directives = parse(&format!(
            "# rubocop:disable {disable}\nfoo(1,2)\n# rubocop:enable {enable}\nfoo(3,4)\n"
        ));
        assert!(directives.suppresses("Layout/SpaceAfterComma", 2));
        assert!(directives.suppresses("Layout/SpaceAfterComma", 3));
        assert!(!directives.suppresses("Layout/SpaceAfterComma", 4));
        assert!(!directives.disabled_by("Layout/SpaceAfterComma", 4, 1));
    }
}
#[test]
fn trailing_enables_do_not_change_block_state_and_trailing_disables_do_not_close_it() {
    let directives = parse(
        "# rubocop:disable Layout\nfoo(1,2) # rubocop:enable Layout\nfoo(3,4) # rubocop:disable Layout/SpaceAfterComma\nfoo(5,6)\n# rubocop:enable all\nfoo(7,8)\n",
    );
    assert!(directives.disabled_by("Layout/SpaceAfterComma", 2, 1));
    assert!(directives.disabled_by("Layout/SpaceAfterComma", 3, 1));
    assert!(directives.disabled_by("Layout/SpaceAfterComma", 3, 3));
    assert!(directives.suppresses("Layout/SpaceAfterComma", 4));
    assert!(!directives.suppresses("Layout/SpaceAfterComma", 6));
}
