use super::{argument_nodes, named_kids};
use tree_sitter::Node;

#[derive(Clone)]
pub struct Argument<'a> {
    pub nodes: Vec<Node<'a>>,
    pub hash: bool,
    pub braces: bool,
}
impl<'a> Argument<'a> {
    pub fn start(&self) -> usize {
        self.nodes[0].start_byte()
    }
    pub fn end(&self) -> usize {
        self.nodes.last().unwrap().end_byte()
    }
    pub fn node(&self) -> Option<Node<'a>> {
        (self.nodes.len() == 1 && (!self.hash || self.braces)).then(|| self.nodes[0])
    }
    pub fn parts(&self) -> Vec<Node<'a>> {
        if self.braces {
            named_kids(self.nodes[0])
                .into_iter()
                .filter(|node| node.kind() != "comment")
                .collect()
        } else {
            self.nodes.clone()
        }
    }
}
pub fn arguments(node: Node<'_>) -> Vec<Argument<'_>> {
    from_nodes(argument_nodes(node))
}
pub fn from_nodes<'a>(nodes: impl IntoIterator<Item = Node<'a>>) -> Vec<Argument<'a>> {
    let mut result = Vec::<Argument<'_>>::new();
    for argument in nodes.into_iter().filter(|node| node.kind() != "comment") {
        if matches!(argument.kind(), "pair" | "hash_splat_argument") {
            if let Some(hash) = result.last_mut().filter(|last| last.hash && !last.braces) {
                hash.nodes.push(argument);
            } else {
                result.push(Argument {
                    nodes: vec![argument],
                    hash: true,
                    braces: false,
                });
            }
        } else {
            result.push(Argument {
                nodes: vec![argument],
                hash: argument.kind() == "hash",
                braces: argument.kind() == "hash",
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::parse::source::SourceFile;
    #[test]
    fn ordinary_arguments_match_original_grouping() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/framework-arguments.json"
        )))
        .unwrap();
        for row in fixture["cases"].as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                row["source"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(!tree.root_node().has_error(), "{}", row["source"]);
            let mut target = None;
            let mut pending = vec![tree.root_node()];
            while let Some(node) = pending.pop() {
                if crate::cop::shared::call_method_name(&source, node) == Some(b"probe") {
                    target = Some(node);
                }
                pending.extend(crate::cop::shared::named_kids(node).into_iter().rev());
            }
            let target = target.unwrap();
            for args in [
                super::arguments(target),
                super::from_nodes(crate::cop::shared::argument_nodes(target)),
            ] {
                let expected = row["arguments"].as_array().unwrap();
                assert_eq!(args.len(), expected.len(), "{}", row["source"]);
                for (arg, want) in args.iter().zip(expected) {
                    assert_eq!(
                        (arg.start(), arg.end()),
                        (
                            want["start"].as_u64().unwrap() as usize,
                            want["end"].as_u64().unwrap() as usize
                        ),
                        "{}",
                        row["source"]
                    );
                    assert_eq!(
                        arg.hash,
                        want["hash"].as_bool().unwrap(),
                        "{}",
                        row["source"]
                    );
                    assert_eq!(
                        arg.braces,
                        want["braces"].as_bool().unwrap(),
                        "{}",
                        row["source"]
                    );
                    assert_eq!(
                        arg.parts().len(),
                        want["parts"].as_u64().unwrap() as usize,
                        "{}",
                        row["source"]
                    );
                    assert_eq!(
                        arg.node().is_some(),
                        !arg.hash || arg.braces,
                        "{}",
                        row["source"]
                    );
                }
            }
        }
    }
}
