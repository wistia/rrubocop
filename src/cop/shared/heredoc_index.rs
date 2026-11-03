use crate::parse::source::SourceFile;
use std::collections::HashMap;
use std::sync::Arc;
use tree_sitter::Node;

pub(crate) struct HeredocNode {
    pub start: usize,
    pub end: usize,
    pub row: usize,
}
pub(crate) struct HeredocBody {
    pub node: HeredocNode,
    pub delimiter: Option<String>,
}
pub(crate) struct HeredocIndex {
    pub openers: Vec<HeredocNode>,
    pub bodies: Vec<HeredocBody>,
    pub interpolations: Vec<usize>,
    unique_bodies: HashMap<String, Option<usize>>,
}
impl HeredocIndex {
    pub fn unique_body(&self, delimiter: &str) -> Option<&HeredocBody> {
        self.unique_bodies
            .get(delimiter)
            .copied()
            .flatten()
            .map(|index| &self.bodies[index])
    }
    pub fn body_for_opener(&self, start: usize) -> Option<&HeredocBody> {
        self.openers
            .binary_search_by_key(&start, |opener| opener.start)
            .ok()
            .and_then(|index| self.bodies.get(index))
    }
    pub fn preceding_on_line(
        &self,
        start: usize,
        row: usize,
    ) -> impl Iterator<Item = &HeredocNode> {
        let first = self.openers.partition_point(|opener| opener.row < row);
        self.openers[first..]
            .iter()
            .take_while(move |opener| opener.row == row && opener.start < start)
    }
}
pub(crate) fn heredoc_index(source: &SourceFile, root: Node<'_>) -> Arc<HeredocIndex> {
    source.cached(root.start_byte(), || {
        let mut index = HeredocIndex {
            openers: Vec::new(),
            bodies: Vec::new(),
            interpolations: Vec::new(),
            unique_bodies: HashMap::new(),
        };
        let mut cursor = root.walk();
        'nodes: loop {
            let node = cursor.node();
            let range = || HeredocNode {
                start: node.start_byte(),
                end: node.end_byte(),
                row: node.start_position().row,
            };
            match node.kind() {
                "heredoc_beginning" => index.openers.push(range()),
                "heredoc_body" => index.bodies.push(HeredocBody {
                    node: range(),
                    delimiter: super::named_kids(node)
                        .into_iter()
                        .find(|child| child.kind() == "heredoc_end")
                        .map(|child| {
                            std::str::from_utf8(super::node_bytes(source, child))
                                .unwrap_or("")
                                .trim()
                                .to_owned()
                        }),
                }),
                "interpolation" => index.interpolations.push(node.start_byte()),
                _ => {}
            }
            if cursor.goto_first_child() {
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    break 'nodes;
                }
            }
        }
        index.openers.sort_by_key(|node| node.start);
        index.bodies.sort_by_key(|body| body.node.start);
        index.interpolations.sort_unstable();
        for (position, body) in index.bodies.iter().enumerate() {
            if let Some(delimiter) = &body.delimiter {
                index
                    .unique_bodies
                    .entry(delimiter.clone())
                    .and_modify(|index| *index = None)
                    .or_insert(Some(position));
            }
        }
        index
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexed_ranges_equal_original_sorted_traversals_for_delayed_and_nested_heredocs() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/core-heredoc-interpolations.json"
        )))
        .unwrap();
        for case in cases.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "heredoc.rb",
                case["source"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            let root = tree.root_node();
            let index = heredoc_index(&source, root);
            assert!(Arc::ptr_eq(&index, &heredoc_index(&source, root)));
            for (kind, actual) in [
                (
                    "heredoc_beginning",
                    index
                        .openers
                        .iter()
                        .map(|node| (node.start, node.end))
                        .collect::<Vec<_>>(),
                ),
                (
                    "heredoc_body",
                    index
                        .bodies
                        .iter()
                        .map(|body| (body.node.start, body.node.end))
                        .collect::<Vec<_>>(),
                ),
            ] {
                let mut expected = Vec::new();
                super::super::for_each_descendant(root, |node| {
                    if node.is_named() && node.kind() == kind {
                        expected.push((node.start_byte(), node.end_byte()));
                    }
                });
                expected.sort_by_key(|range| range.0);
                assert_eq!(actual, expected, "{} {kind}", case["id"]);
            }
            for opener in &index.openers {
                let expected = index
                    .openers
                    .iter()
                    .filter(|candidate| {
                        candidate.row == opener.row && candidate.start < opener.start
                    })
                    .map(|node| node.start)
                    .collect::<Vec<_>>();
                let actual = index
                    .preceding_on_line(opener.start, opener.row)
                    .map(|node| node.start)
                    .collect::<Vec<_>>();
                assert_eq!(actual, expected);
                let pair = index
                    .openers
                    .iter()
                    .position(|candidate| candidate.start == opener.start)
                    .and_then(|position| index.bodies.get(position));
                assert_eq!(
                    index
                        .body_for_opener(opener.start)
                        .map(|body| (body.node.start, body.node.end)),
                    pair.map(|body| (body.node.start, body.node.end))
                );
            }
            for body in &index.bodies {
                if let Some(delimiter) = &body.delimiter {
                    let matching = index
                        .bodies
                        .iter()
                        .filter(|candidate| candidate.delimiter.as_ref() == Some(delimiter))
                        .collect::<Vec<_>>();
                    let expected = (matching.len() == 1).then(|| matching[0].node.start);
                    assert_eq!(
                        index.unique_body(delimiter).map(|body| body.node.start),
                        expected
                    );
                }
            }
            let mut interpolations = Vec::new();
            super::super::for_each_descendant(root, |node| {
                if node.is_named() && node.kind() == "interpolation" {
                    interpolations.push(node.start_byte());
                }
            });
            interpolations.sort_unstable();
            assert_eq!(index.interpolations, interpolations, "{}", case["id"]);
        }
    }
}
