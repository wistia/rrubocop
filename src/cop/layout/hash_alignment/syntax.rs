use super::geometry::{Element, line};
use crate::cop::CopConfig;
use crate::cop::shared::named_kids;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) struct Group<'a> {
    pub(super) elements: Vec<Element<'a>>,
    pub(super) braced: bool,
    pub(super) parent: Option<Node<'a>>,
    pub(super) previous: Option<Node<'a>>,
    pub(super) last: bool,
}
pub(super) fn group(node: Node<'_>) -> Option<Group<'_>> {
    if node.kind() != "hash" && node.has_error() {
        return None;
    }
    let children: Vec<_> = named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect();
    let explicit = node.kind() == "hash";
    let first = children.iter().position(|n| Element::new(*n).is_some())?;
    let last = children.iter().rposition(|n| Element::new(*n).is_some())?;
    let elements = children[first..=last]
        .iter()
        .filter_map(|n| Element::new(*n))
        .collect();
    if explicit {
        let parent = node.parent();
        let in_args =
            parent.is_some_and(|n| matches!(n.kind(), "argument_list" | "command_argument_list"));
        if let Some(assignment) = parent.filter(|p| p.kind() == "assignment") {
            if let Some(left) = assignment
                .child_by_field_name("left")
                .filter(|n| matches!(n.kind(), "call" | "element_reference"))
            {
                let previous = if left.kind() == "element_reference" {
                    named_kids(left)
                        .into_iter()
                        .filter(|n| Some(*n) != left.child_by_field_name("object"))
                        .last()
                } else {
                    left.child_by_field_name("arguments").and_then(|args| {
                        named_kids(args)
                            .into_iter()
                            .filter(|n| n.kind() != "comment")
                            .last()
                    })
                };
                return Some(Group {
                    elements,
                    braced: true,
                    parent: Some(left),
                    previous,
                    last: true,
                });
            }
        }
        let siblings: Vec<_> = parent
            .map(named_kids)
            .unwrap_or_default()
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .collect();
        let index = siblings.iter().position(|n| n.id() == node.id());
        Some(Group {
            elements,
            braced: true,
            parent: if in_args {
                parent.and_then(|n| n.parent())
            } else {
                None
            },
            previous: index.and_then(|i| i.checked_sub(1)).map(|i| siblings[i]),
            last: in_args && index == siblings.len().checked_sub(1),
        })
    } else {
        Some(Group {
            elements,
            braced: false,
            parent: if matches!(node.kind(), "argument_list" | "command_argument_list") {
                node.parent()
            } else if node.kind() == "element_reference" {
                Some(node)
            } else {
                None
            },
            previous: first.checked_sub(1).map(|i| children[i]),
            last: last + 1 == children.len(),
        })
    }
}
pub(super) fn skip(
    source: &SourceFile,
    group: &Group<'_>,
    first: Element<'_>,
    config: &CopConfig,
) -> bool {
    let parent = group.parent;
    let call = parent.is_some_and(|n| {
        matches!(
            n.kind(),
            "call" | "command" | "command_call" | "element_reference"
        )
    });
    let special = parent.is_some_and(|n| {
        matches!(n.kind(), "super" | "yield")
            || n.child_by_field_name("method")
                .is_some_and(|m| matches!(m.kind(), "super" | "yield"))
    });
    let dispatch = call || special;
    let call = call && !special;
    if dispatch && group.last {
        match config.get_str("EnforcedLastArgumentHashStyle", "always_inspect") {
            "always_ignore" => return true,
            "ignore_explicit" if group.braced => return true,
            "ignore_implicit" if !group.braced => return true,
            _ => {}
        }
    }
    if call
        && config.get_bool("ArgumentAlignmentEnabled", true)
        && config.get_str("ArgumentAlignmentStyle", "with_first_argument")
            == "with_fixed_indentation"
    {
        let selector = group
            .previous
            .or_else(|| parent.and_then(|n| n.child_by_field_name("method")))
            .or(parent);
        if selector.is_some_and(|n| line(source, n) == line(source, first.node)) {
            return true;
        }
    }
    false
}
