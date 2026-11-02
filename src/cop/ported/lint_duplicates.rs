mod literal_key;
use literal_key::{literal_key, require_key};
pub(super) fn expression_key(c: &Context<'_>, n: Node<'_>) -> String {
    literal_key(c, n)
}
mod empty_when;
use super::context::*;
use crate::cop::CopRegistry;
use empty_when::empty_when;
use std::collections::HashSet;
use tree_sitter::Node;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Lint/DuplicateCaseCondition", &["case"], case_conditions, false, true, false),
        ("Lint/DuplicateElsifCondition", &["if"], elsif_conditions, false, true, false),
        ("Lint/DuplicateRescueException", &["body_statement","begin"], rescue_exceptions, false, true, false),
        ("Lint/DuplicateRequire", &["call"], require, true, false, false),
        ("Lint/EmptyWhen", &["when"], empty_when, false, true, false)
    );
}
fn duplicate(c: &Context<'_>, nodes: Vec<Node<'_>>, msg: &str) -> Vec<Finding> {
    let mut seen = HashSet::new();
    nodes
        .into_iter()
        .filter(|n| !seen.insert(literal_key(c, *n)))
        .map(|n| Finding::new(n, msg))
        .collect()
}
fn case_conditions(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    duplicate(
        c,
        kids(n)
            .into_iter()
            .filter(|w| w.kind() == "when")
            .flat_map(|w| {
                let mut cursor = w.walk();
                w.children_by_field_name("pattern", &mut cursor)
                    .collect::<Vec<_>>()
            })
            .collect(),
        "Duplicate `when` condition detected.",
    )
}
fn elsif_conditions(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let mut nodes = vec![];
    let mut next = Some(n);
    while let Some(node) = next {
        if let Some(cond) = field(node, "condition") {
            nodes.push(cond);
        }
        next = field(node, "alternative").filter(|p| p.kind() == "elsif");
    }
    duplicate(c, nodes, "Duplicate `elsif` condition detected.")
}
fn rescue_exceptions(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    duplicate(
        c,
        kids(n)
            .into_iter()
            .filter(|r| r.kind() == "rescue")
            .filter_map(|r| field(r, "exceptions"))
            .flat_map(kids)
            .collect(),
        "Duplicate `rescue` exception detected.",
    )
}
fn require(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !require_call(c, n) || !repeated_require(c, n) {
        return vec![];
    }
    let (start, end) = whole_lines(c, n);
    vec![
        Finding::new(n, format!("Duplicate `{}` detected.", c.method(n))).edit_with(
            c,
            start,
            end,
            || "",
        ),
    ]
}
fn require_call(c: &Context<'_>, n: Node<'_>) -> bool {
    ["require", "require_relative"].contains(&c.method(n))
        && kernel_receiver(c, n)
        && semantic_args(n).len() == 1
}
struct RepeatedRequires(HashSet<usize>);
fn repeated_require(c: &Context<'_>, n: Node<'_>) -> bool {
    c.source
        .cached(0, || index_requirements(c, n))
        .0
        .contains(&n.start_byte())
}
fn index_requirements(c: &Context<'_>, n: Node<'_>) -> RepeatedRequires {
    let root = tree_root(n);
    let mut seen = HashSet::new();
    RepeatedRequires(
        descendants(root)
            .filter(|node| require_candidate(c, *node))
            .filter(|node| !seen.insert(requirement_key(c, *node)))
            .map(|node| node.start_byte())
            .collect(),
    )
}

fn tree_root(n: Node<'_>) -> Node<'_> {
    ancestors(n).last().unwrap_or(n)
}
fn require_candidate(c: &Context<'_>, n: Node<'_>) -> bool {
    is_call(n) && require_call(c, n)
}
fn requirement_key<'a>(
    c: &'a Context<'_>,
    node: Node<'_>,
) -> (usize, usize, &'static str, &'a str, String) {
    let parent = ruby_parent(node);
    (
        parent.start_byte(),
        parent.end_byte(),
        parent.kind(),
        c.method(node),
        require_key(c, semantic_args(node)[0]),
    )
}
fn ruby_parent(n: Node<'_>) -> Node<'_> {
    let parent = n.parent().unwrap_or(n);
    if matches!(parent.kind(), "then" | "else") && kids(parent).len() == 1 {
        parent.parent().unwrap_or(parent)
    } else {
        parent
    }
}

fn whole_lines(c: &Context<'_>, n: Node<'_>) -> (usize, usize) {
    (
        c.source
            .line_start(n.start_position().row + 1)
            .unwrap_or(n.start_byte()),
        c.source
            .line_start(n.end_position().row + 2)
            .unwrap_or(c.source.as_bytes().len()),
    )
}
fn kernel_receiver(c: &Context<'_>, n: Node<'_>) -> bool {
    field(n, "receiver").is_none_or(|r| {
        matches!(r.kind(), "constant" | "scope_resolution")
            && c.text(r).rsplit("::").next() == Some("Kernel")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requirements_are_indexed_in_order_with_ruby_parent_boundaries() {
        let source = crate::parse::source::SourceFile::from_bytes("sample.rb", b"require 'a'\n# comment\nrequire 'a'\nrequire_relative 'a'\nif condition\n require 'a'\n require 'a'\nend\nrequire 'b'\n".to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = crate::cop::CopConfig::default();
        let c = Context::new(&source, &config);
        let calls = descendants(tree.root_node())
            .filter(|n| is_call(*n) && require_call(&c, *n))
            .collect::<Vec<_>>();
        assert_eq!(
            calls
                .iter()
                .map(|n| repeated_require(&c, *n))
                .collect::<Vec<_>>(),
            [false, true, false, false, true, false]
        );
    }
}

#[cfg(test)]
mod decode_tests;
