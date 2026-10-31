//! Assignment-LHS / index-write guards for RedundantSelf.

use tree_sitter::Node;

pub(super) fn call_is_assign_lhs(node: Node<'_>) -> bool {
    node.parent().is_some_and(|p| {
        if p.kind() == "left_assignment_list" {
            return true;
        }
        matches!(p.kind(), "assignment" | "operator_assignment")
            && p.child_by_field_name("left")
                .is_some_and(|l| l.id() == node.id())
    })
}

/// `self.rates[id] ||= {}` — walk through element_reference to assignment.
pub(super) fn call_is_index_assign_recv(node: Node<'_>) -> bool {
    let mut cur = node;
    for _ in 0..4 {
        let Some(parent) = cur.parent() else {
            return false;
        };
        if matches!(parent.kind(), "assignment" | "operator_assignment") {
            return parent
                .child_by_field_name("left")
                .is_some_and(|l| l.id() == cur.id() || contains_node(l, node));
        }
        if !advance_index_recv(&mut cur, parent) {
            return false;
        }
    }
    false
}

fn advance_index_recv<'a>(cur: &mut Node<'a>, parent: Node<'a>) -> bool {
    if !matches!(parent.kind(), "element_reference" | "call") {
        return false;
    }
    if parent
        .child_by_field_name("object")
        .or_else(|| parent.child_by_field_name("receiver"))
        .is_some_and(|r| r.id() == cur.id())
    {
        *cur = parent;
        true
    } else {
        false
    }
}

fn contains_node(root: Node<'_>, target: Node<'_>) -> bool {
    if root.id() == target.id() {
        return true;
    }
    let mut cur = root.walk();
    root.named_children(&mut cur)
        .any(|c| contains_node(c, target))
}
