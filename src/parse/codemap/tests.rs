use super::*;
use crate::parse::{parse_ruby, source::SourceFile};

fn assert_equivalent(ranges: Vec<(usize, usize)>, offsets: impl IntoIterator<Item = usize>) {
    let map = CodeMap {
        ranges: union(ranges.clone()),
    };
    assert_map(&map, &ranges, offsets);
}

fn assert_map(map: &CodeMap, ranges: &[(usize, usize)], offsets: impl IntoIterator<Item = usize>) {
    assert!(map.ranges.iter().all(|&(start, end)| start < end));
    assert!(map.ranges.windows(2).all(|pair| pair[0].1 < pair[1].0));
    for offset in offsets {
        let expected = ranges.iter().any(|&(s, e)| s <= offset && offset < e);
        assert_eq!(map.covers(offset), expected, "{ranges:?} at {offset}");
    }
}

#[test]
fn range_union_handles_nested_overlapping_touching_empty_and_reversed_intervals() {
    for ranges in [
        vec![],
        vec![(3, 3), (8, 2)],
        vec![(0, 20), (4, 8), (6, 10)],
        vec![(5, 9), (1, 5), (12, 15), (9, 12)],
        vec![(20, 30), (1, 8), (4, 12), (1, 8), (14, 16)],
    ] {
        assert_equivalent(ranges, (0..=32).rev());
    }
}

#[test]
fn all_small_interval_triples_preserve_every_byte_membership() {
    let intervals = (0..=5)
        .flat_map(|start| (0..=5).map(move |end| (start, end)))
        .collect::<Vec<_>>();
    for first in &intervals {
        for second in &intervals {
            for third in &intervals {
                assert_equivalent(vec![*first, *second, *third], 0..=6);
            }
        }
    }
}

#[test]
fn exclusive_ends_and_maximum_offsets_do_not_overflow() {
    assert_equivalent(
        vec![
            (0, 0),
            (usize::MAX, usize::MAX),
            (usize::MAX - 2, usize::MAX),
        ],
        [
            0,
            usize::MAX - 3,
            usize::MAX - 2,
            usize::MAX - 1,
            usize::MAX,
        ],
    );
}

#[test]
fn ruby_regions_match_their_normalized_ranges_in_any_query_order() {
    for text in [
        "a = 'a,b'; b = \"#{call(1, 2)}\" # comment\n",
        "a = /(?<name>a)[b,]/x; b = %r{a#{value}b}; c = ?x\n",
        "a = %w[a b]; b = %W[a #{value}]; c = %i[a b]; d = %I[a #{value}]\n",
        "a = %q[a,b]; b = %Q[a#{value}]; c = %x[echo a]; d = %s[a,b]\n",
        "a = {name: :value, 'Ü': \"🌟,#{value}\"}\r\n# Ü comment\r\n",
        "outer(<<~FIRST, <<~SECOND)\n #{inner(<<~NESTED)}\n  nested\n NESTED\nFIRST\n second\nSECOND\n",
        "a = <<'CODE'\n# literal, #{ignored}\nCODE\nputs :done\n",
    ] {
        assert_source(text);
    }
}

fn assert_source(text: &str) {
    let source = SourceFile::from_bytes("example.rb", text.as_bytes().to_vec());
    let tree = parse_ruby(&source).unwrap();
    let root = tree.root_node();
    assert!(!root.has_error(), "{text}");
    let mut ranges = Vec::new();
    collect_ranges(root, source.as_bytes(), &mut ranges);
    let map = CodeMap::from_tree(root, source.as_bytes());
    assert_map(&map, &ranges, (0..=text.len() + 1).rev());
}

#[test]
fn interpolation_delimiters_and_nested_literal_text_remain_covered() {
    let text = "puts \"Ü,#{foo(1,2,\"🌟,#{bar(3,4)}\")};#{first;second}#@var\"\n";
    let source = SourceFile::from_bytes("example.rb", text.as_bytes().to_vec());
    let tree = parse_ruby(&source).unwrap();
    assert!(!tree.root_node().has_error());
    let map = CodeMap::from_tree(tree.root_node(), source.as_bytes());
    assert_literal_coverage(text, &map);
    for code in ["foo(1,2,", "bar(3,4)", "first;second"] {
        let start = text.find(code).unwrap();
        for offset in start..start + code.len() {
            assert!(!map.covers(offset), "{code} at {offset}");
        }
    }
}

fn assert_literal_coverage(text: &str, map: &CodeMap) {
    for literal in ["Ü,", "🌟,", "#{", "}", "#"] {
        for (start, matched) in text.match_indices(literal) {
            for offset in start..start + matched.len() {
                assert!(map.covers(offset), "{literal} at {offset}");
            }
        }
    }
}

#[test]
fn literal_containers_cover_their_punctuation_delimiters() {
    for text in [
        "%s;foo;", "%s,foo ,", "%i,a b,", "%x,echo,", "%w,a b,", ":'a,b'",
    ] {
        let source = SourceFile::from_bytes("t.rb", text.as_bytes().to_vec());
        let tree = parse_ruby(&source).unwrap();
        assert!(!tree.root_node().has_error(), "{text}");
        let map = CodeMap::from_tree(tree.root_node(), source.as_bytes());
        assert!((0..text.len()).all(|offset| map.covers(offset)), "{text}");
    }
}
#[test]
fn noninterpolating_literal_prefixes_prune_only_actual_literal_containers() {
    for (text, permitted) in [
        ("%w(a b)", false),
        ("%i(a b)", false),
        ("%q(a)", false),
        ("%s(a)", false),
        ("'a'", false),
        ("%W(a)", true),
        ("%I(a)", true),
        ("%Q(a)", true),
        ("%x(a)", true),
    ] {
        let source = SourceFile::from_bytes("t.rb", text.as_bytes().to_vec());
        let tree = parse_ruby(&source).unwrap();
        let node = tree.root_node().named_child(0).unwrap();
        assert_eq!(
            permits_interpolation(node, source.as_bytes()),
            permitted,
            "{text}"
        );
    }
}
