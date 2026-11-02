use super::parity_helpers::{call_args, code_children};
use tree_sitter::Node;

#[derive(Clone)]
pub(super) struct Argument<'a> {
    pub(super) nodes: Vec<Node<'a>>,
    pub(super) hash: bool,
    pub(super) braces: bool,
}
impl<'a> Argument<'a> {
    pub(super) fn start(&self) -> usize {
        self.nodes[0].start_byte()
    }
    pub(super) fn end(&self) -> usize {
        self.nodes.last().unwrap().end_byte()
    }
    pub(super) fn node(&self) -> Option<Node<'a>> {
        (self.nodes.len() == 1).then(|| self.nodes[0])
    }
    pub(super) fn parts(&self) -> Vec<Node<'a>> {
        if self.braces {
            code_children(self.nodes[0])
        } else {
            self.nodes.clone()
        }
    }
}
pub(super) fn arguments(node: Node<'_>) -> Vec<Argument<'_>> {
    let mut result = Vec::<Argument<'_>>::new();
    for argument in call_args(node) {
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
