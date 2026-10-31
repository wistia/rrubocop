use tree_sitter::Node;

pub(in crate::cop::ported) fn descendants(node: Node<'_>) -> Descendants<'_> {
    Descendants {
        root: node,
        next: Some(node),
    }
}

pub(in crate::cop::ported) struct Descendants<'a> {
    root: Node<'a>,
    next: Option<Node<'a>>,
}

impl<'a> Iterator for Descendants<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.next?;
        self.next = node.named_child(0).or_else(|| {
            let mut current = node;
            while current != self.root {
                if let Some(sibling) = current.next_named_sibling() {
                    return Some(sibling);
                }
                current = current.parent()?;
            }
            None
        });
        Some(node)
    }
}

pub(in crate::cop::ported) fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(node.parent(), |parent| parent.parent())
}
