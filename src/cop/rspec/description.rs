use super::parity_helpers::*;
use crate::cop::shared::node_bytes;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn string(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "string" | "chained_string" | "heredoc_beginning" | "character"
    )
}
fn find<'a>(root: Node<'a>, start: usize, kind: &str) -> Option<Node<'a>> {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.start_byte() == start && node.kind() == kind {
            return Some(node);
        }
        pending.extend(code_children(node));
    }
    None
}
fn static_text(source: &SourceFile, node: Node<'_>) -> Option<String> {
    crate::cop::ported::static_literal(source, node)
        .or_else(|| crate::cop::ported::ruby_string_parts(source, node).map(|parts| parts.concat()))
}
struct InterpolationIndex(Vec<super::cache::NodeRange>);
pub(super) fn interpolations<'a>(source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    let range = if node.kind() == "heredoc_beginning" {
        crate::cop::ported::heredoc_span(source, node)
    } else {
        Some((node.start_byte(), node.end_byte()))
    };
    let Some((start, end)) = range else {
        return Vec::new();
    };
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let index = source.cached(0, || {
        let mut pending = vec![root];
        let mut ranges = Vec::new();
        while let Some(child) = pending.pop() {
            if child.kind() == "interpolation" {
                ranges.push(super::cache::NodeRange::new(child));
            }
            pending.extend(code_children(child));
        }
        ranges.sort_by_key(|range| (range.start, std::cmp::Reverse(range.end)));
        InterpolationIndex(ranges)
    });
    let first = index.0.partition_point(|range| range.start < start);
    let mut last_end = start;
    index.0[first..]
        .iter()
        .take_while(|range| range.start < end)
        .filter_map(|range| {
            if range.end > end || range.start < last_end {
                return None;
            }
            let node = range.resolve(root)?;
            last_end = range.end;
            Some(node)
        })
        .collect()
}
#[derive(Clone)]
struct DescriptionText(Option<String>);
pub(super) fn text(source: &SourceFile, node: Node<'_>) -> Option<String> {
    super::cache::node_fact(source, node, || {
        DescriptionText(uncached_text(source, node))
    })
    .0
}
fn marker_value(
    source: &SourceFile,
    node: Node<'_>,
    expressions: &[Node<'_>],
    markers: &[String],
    fragment: bool,
) -> Option<String> {
    let offset = if fragment { node.start_byte() } else { 0 };
    let mut bytes = if fragment {
        node_bytes(source, node).to_vec()
    } else {
        source.as_bytes().to_vec()
    };
    for (expression, marker) in expressions.iter().zip(markers).rev() {
        bytes.splice(
            expression.start_byte() - offset..expression.end_byte() - offset,
            marker.bytes(),
        );
    }
    let surrogate = SourceFile::from_bytes("description.rb", bytes);
    let tree = crate::parse::parse_ruby(&surrogate).ok()?;
    if tree.root_node().has_error() {
        return None;
    }
    let literal = find(tree.root_node(), node.start_byte() - offset, node.kind())?;
    static_text(&surrogate, literal)
}
fn uncached_text(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if !string(node) {
        return None;
    }
    if let Some(text) = static_text(source, node) {
        return Some(text);
    }
    let expressions = interpolations(source, node);
    if expressions.is_empty() {
        return None;
    }
    for salt in 0..=source.as_bytes().len() {
        let markers: Vec<_> = (0..expressions.len())
            .map(|index| format!("RRUBOCOP_INTERPOLATION_{salt}_{index}_END"))
            .collect();
        let value = (node.kind() != "heredoc_beginning")
            .then(|| marker_value(source, node, &expressions, &markers, true))
            .flatten()
            .or_else(|| marker_value(source, node, &expressions, &markers, false))?;
        if markers
            .iter()
            .any(|marker| value.matches(marker.as_str()).count() != 1)
        {
            continue;
        }
        let mut replacements: Vec<_> = expressions
            .iter()
            .zip(&markers)
            .map(|(expression, marker)| {
                (
                    value.find(marker).unwrap(),
                    marker.len(),
                    node_bytes(source, *expression),
                )
            })
            .collect();
        replacements.sort_by_key(|(start, _, _)| *start);
        let mut result = String::new();
        let mut cursor = 0;
        for (start, length, raw) in replacements {
            result.push_str(&value[cursor..start]);
            result.push_str(std::str::from_utf8(raw).ok()?);
            cursor = start + length;
        }
        result.push_str(&value[cursor..]);
        return Some(result);
    }
    None
}
pub(super) fn first_static(source: &SourceFile, node: Node<'_>, symbols: bool) -> bool {
    if node.kind() == "chained_string" {
        return code_children(node)
            .first()
            .is_some_and(|child| string(*child));
    }
    if let Some(first) = code_children(node).first().copied() {
        if first.kind() != "interpolation" {
            return true;
        }
        if symbols {
            let mut pending = vec![first];
            while let Some(node) = pending.pop() {
                if symbol_node(node)
                    && crate::cop::ported::static_symbol(source, node).is_some()
                    && crate::cop::ported::ruby_string_parts(source, node)
                        .is_none_or(|parts| parts.len() == 1)
                {
                    return true;
                }
                pending.extend(code_children(node));
            }
        }
        return false;
    }
    true
}
fn line_tokens(source: &SourceFile, expression: Node<'_>, start_line: usize) -> Vec<usize> {
    let mut pending = vec![expression];
    let mut result = Vec::new();
    while let Some(node) = pending.pop() {
        if node.kind() == "identifier" && node_bytes(source, node) == b"__LINE__" {
            result.push(source.offset_to_line_col(node.start_byte()).0 - start_line);
        }
        pending.extend(code_children(node));
    }
    result.sort_unstable();
    result
}
fn expression_body(bytes: &[u8]) -> Option<&str> {
    let body = bytes
        .strip_prefix(b"#{")
        .and_then(|body| body.strip_suffix(b"}"))
        .or_else(|| bytes.strip_prefix(b"#"))?;
    std::str::from_utf8(body).ok()
}
fn equivalent_expression(old: &[u8], new: &[u8]) -> bool {
    if old == new {
        return true;
    }
    let Some(old) = expression_body(old) else {
        return false;
    };
    let Some(new) = expression_body(new) else {
        return false;
    };
    let source = SourceFile::from_bytes(
        "interpolation.rb",
        format!("[({old}), ({new})]").into_bytes(),
    );
    let Ok(tree) = crate::parse::parse_ruby(&source) else {
        return false;
    };
    if tree.root_node().has_error() {
        return false;
    }
    let Some(array) = tree.root_node().named_child(0) else {
        return false;
    };
    let nodes = code_children(array);
    nodes.len() == 2
        && crate::cop::shared::ruby_expression::eql_expression(&source, nodes[0], nodes[1])
}
pub(super) fn safe_replacement(source: &SourceFile, node: Node<'_>, replacement: &str) -> bool {
    if node.kind() == "heredoc_beginning" || node.end_byte() < node.start_byte() + 2 {
        return false;
    }
    let mut bytes = node_bytes(source, node).to_vec();
    bytes.splice(1..bytes.len() - 1, replacement.bytes());
    let candidate = SourceFile::from_bytes("description.rb", bytes);
    let Ok(tree) = crate::parse::parse_ruby(&candidate) else {
        return false;
    };
    let root = tree.root_node();
    if root.has_error() || root.named_child_count() != 1 {
        return false;
    }
    let Some(literal) = root.named_child(0).filter(|node| string(*node)) else {
        return false;
    };
    let old: Vec<_> = interpolations(source, node)
        .iter()
        .map(|node| node_bytes(source, *node))
        .collect();
    let new_nodes = interpolations(&candidate, literal);
    let new: Vec<_> = new_nodes
        .iter()
        .map(|node| node_bytes(&candidate, *node))
        .collect();
    old.len() == new.len()
        && old
            .iter()
            .zip(&new)
            .all(|(old, new)| equivalent_expression(old, new))
        && interpolations(source, node)
            .iter()
            .zip(&new_nodes)
            .all(|(old, new)| {
                line_tokens(source, *old, source.offset_to_line_col(node.start_byte()).0)
                    == line_tokens(
                        &candidate,
                        *new,
                        candidate.offset_to_line_col(literal.start_byte()).0,
                    )
            })
        && text(&candidate, literal).as_deref() == Some(replacement)
}
pub(super) fn blank(character: char) -> bool {
    matches!(
        character,
        '\t' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

#[cfg(test)]
mod efficiency_tests {
    use super::*;

    #[test]
    fn fragment_decoding_matches_full_source_surrogates_for_nested_and_multiline_descriptions() {
        for input in [
            "describe A do\n it \"should #{name} run\" do; work; end\nend\n",
            "# encoding: UTF-8\r\ndescribe A do\r\n it %Q(should #{\"#{name}\"} run) do; work; end\r\nend\r\n",
            "describe A do; it \"should RRUBOCOP_INTERPOLATION_0_0_END #{__LINE__} run\" do; work; end; end",
            "describe A do; it \"should #{name}\" ' run' do; work; end; end",
            "describe A do; it \"should \\\n#{name} run\" do; work; end; end",
        ] {
            let source = SourceFile::from_bytes("spec/a_spec.rb", input.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(!tree.root_node().has_error(), "{input}");
            let mut nodes = vec![tree.root_node()];
            while let Some(node) = nodes.pop() {
                nodes.extend(code_children(node));
                if !string(node) {
                    continue;
                }
                let expressions = interpolations(&source, node);
                if expressions.is_empty() {
                    continue;
                }
                let markers: Vec<_> = (0..expressions.len())
                    .map(|i| format!("MARKER_{i}_END"))
                    .collect();
                assert_eq!(
                    marker_value(&source, node, &expressions, &markers, true),
                    marker_value(&source, node, &expressions, &markers, false),
                    "{input}: {}",
                    node.kind()
                );
                assert_eq!(text(&source, node), uncached_text(&source, node));
            }
        }
    }
}
