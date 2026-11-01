use tree_sitter::{Node, TreeCursor};

pub(in crate::cop::ported) fn descendants(node: Node<'_>) -> Descendants<'_> {
    Descendants {
        cursor: node.walk(),
        done: false,
        first: true,
    }
}

pub(in crate::cop::ported) struct Descendants<'a> {
    cursor: TreeCursor<'a>,
    done: bool,
    first: bool,
}

impl Descendants<'_> {
    fn advance(&mut self) {
        if self.cursor.goto_first_child() {
            return;
        }
        loop {
            if self.cursor.goto_next_sibling() {
                return;
            }
            if !self.cursor.goto_parent() {
                self.done = true;
                return;
            }
        }
    }
}

impl<'a> Iterator for Descendants<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.done {
            let first = std::mem::replace(&mut self.first, false);
            let node = self.cursor.node();
            self.advance();
            if first || node.is_named() {
                return Some(node);
            }
        }
        None
    }
}

pub(in crate::cop::ported) fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(node.parent(), |parent| parent.parent())
}
