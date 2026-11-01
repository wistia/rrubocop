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
