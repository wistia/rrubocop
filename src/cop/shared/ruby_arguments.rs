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
    let mut result = Vec::<Argument<'_>>::new();
    for argument in argument_nodes(node)
        .into_iter()
        .filter(|node| node.kind() != "comment")
    {
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
