//! Helpers for Style/TrailingCommaIn*Literal (and Arguments).

use tree_sitter::Node;

use crate::parse::source::SourceFile;

pub(crate) fn begins_its_line(source: &SourceFile, offset: usize) -> bool {
    crate::cop::shared::line_indent(source, offset) == source.offset_to_line_col(offset).1
}

fn end_line(source: &SourceFile, start: usize, end: usize) -> usize {
    source
        .offset_to_line_col(end.saturating_sub(1).max(start))
        .0
}

fn spans_lines(source: &SourceFile, start: usize, end: usize) -> bool {
    source.offset_to_line_col(start).0 != end_line(source, start, end)
}

fn push_pair_group(source: &SourceFile, group: &[Node<'_>], out: &mut Vec<(usize, usize)>) {
    let g_start = group[0].start_byte();
    let g_end = group[group.len() - 1].end_byte();
    if spans_lines(source, g_start, g_end) {
        out.extend(group.iter().map(|n| (n.start_byte(), n.end_byte())));
    } else {
        out.push((g_start, g_end));
    }
}

fn hash_pairs(n: Node<'_>) -> Vec<(usize, usize)> {
    let mut cur = n.walk();
    n.named_children(&mut cur)
        .filter(|c| c.kind() == "pair")
        .map(|c| (c.start_byte(), c.end_byte()))
        .collect()
}

fn push_hash_arg(source: &SourceFile, n: Node<'_>, out: &mut Vec<(usize, usize)>) {
    let braced = source.as_bytes().get(n.start_byte()) == Some(&b'{');
    if braced || !spans_lines(source, n.start_byte(), n.end_byte()) {
        out.push((n.start_byte(), n.end_byte()));
    } else {
        out.extend(hash_pairs(n));
    }
}

/// RuboCop `elements(node)` — expand only multiline braceless kw hashes.
pub(crate) fn effective_locs(source: &SourceFile, args: &[Node<'_>]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i].kind() == "pair" {
            let start = i;
            while i < args.len() && args[i].kind() == "pair" {
                i += 1;
            }
            push_pair_group(source, &args[start..i], &mut out);
        } else if args[i].kind() == "hash" {
            push_hash_arg(source, args[i], &mut out);
            i += 1;
        } else {
            out.push((args[i].start_byte(), args[i].end_byte()));
            i += 1;
        }
    }
    out
}

/// RuboCop `no_elements_on_same_line?`.
pub(crate) fn each_elem_own_line(
    source: &SourceFile,
    locs: &[(usize, usize)],
    close: usize,
) -> bool {
    for w in locs.windows(2) {
        if end_line(source, w[0].0, w[0].1) == source.offset_to_line_col(w[1].0).0 {
            return false;
        }
    }
    match locs.last() {
        Some(&(s, e)) if end_line(source, s, e) == source.offset_to_line_col(close).0 => false,
        Some(_) => true,
        None => false,
    }
}

pub(crate) fn should_have_comma(
    source: &SourceFile,
    locs: &[(usize, usize)],
    style: &str,
    close: usize,
) -> bool {
    if locs.is_empty() || (locs.len() == 1 && !begins_its_line(source, close)) {
        return false;
    }
    match style {
        "comma" => each_elem_own_line(source, locs, close),
        "consistent_comma" => true,
        _ => false,
    }
}
