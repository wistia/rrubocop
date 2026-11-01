use tree_sitter::{Node, TreeCursor};

pub(in crate::cop::ported) fn option_pairs(node: Node<'_>) -> Pairs<'_> {
    let (single, arguments, hash) = match node.kind() {
        "pair" => (Some(node), None, None),
        "hash" => (None, None, Some(Children::new(node))),
        "call" => (
            None,
            node.child_by_field_name("arguments").map(Children::new),
            None,
        ),
        _ => (None, None, None),
    };
    Pairs {
        single,
        arguments,
        hash,
    }
}
pub(in crate::cop::ported) struct Pairs<'a> {
    single: Option<Node<'a>>,
    arguments: Option<Children<'a>>,
    hash: Option<Children<'a>>,
}
impl<'a> Iterator for Pairs<'a> {
    type Item = Node<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pair) = self.single.take() {
            return Some(pair);
        }
        loop {
            if let Some(pair) = self.next_hash() {
                return Some(pair);
            }
            self.hash = None;
            match self.arguments.as_mut()?.next()? {
                n if n.kind() == "pair" => return Some(n),
                n if n.kind() == "hash" => self.hash = Some(Children::new(n)),
                _ => {}
            }
        }
    }
}
struct Children<'a> {
    cursor: TreeCursor<'a>,
    started: bool,
}
impl<'a> Children<'a> {
    fn new(n: Node<'a>) -> Self {
        Self {
            cursor: n.walk(),
            started: false,
        }
    }
}
impl<'a> Iterator for Children<'a> {
    type Item = Node<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let moved = if self.started {
                self.cursor.goto_next_sibling()
            } else {
                self.started = true;
                self.cursor.goto_first_child()
            };
            if !moved {
                return None;
            }
            let n = self.cursor.node();
            if n.is_named() {
                return Some(n);
            }
        }
    }
}

impl<'a> Pairs<'a> {
    fn next_hash(&mut self) -> Option<Node<'a>> {
        self.hash.as_mut()?.find(|n| n.kind() == "pair")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::ported::context::{args, descendants, kids};
    use crate::parse::source::SourceFile;

    fn old_pairs(n: Node<'_>) -> Vec<Node<'_>> {
        match n.kind() {
            "pair" => vec![n],
            "hash" => kids(n).into_iter().filter(|n| n.kind() == "pair").collect(),
            "call" => args(n)
                .into_iter()
                .filter(|n| matches!(n.kind(), "hash" | "pair"))
                .flat_map(old_pairs)
                .collect(),
            _ => vec![],
        }
    }
    #[test]
    fn lazy_options_keep_direct_argument_boundaries_order_and_comments() {
        let source = SourceFile::from_bytes(
            "options.rb",
            b"foo({a: 1}, {\n# note\nb: 2, c: nested(d: 3)}, e: 4)\n".to_vec(),
        );
        let tree = crate::parse::parse_ruby(&source).unwrap();
        for n in descendants(tree.root_node()) {
            assert_eq!(
                option_pairs(n).map(|n| n.id()).collect::<Vec<_>>(),
                old_pairs(n).iter().map(|n| n.id()).collect::<Vec<_>>()
            );
        }
    }
}
