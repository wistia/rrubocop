use crate::cop::layout::original_alignment::Item;
use crate::cop::shared::{self, UnaryCommandCall, argument_nodes, named_kids, node_bytes};
use crate::cop::{CopConfig, ported};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

struct Hash {
    braces: bool,
    pairs: Vec<Item>,
}
struct Argument {
    span: Item,
    hash: Option<Hash>,
}
pub(super) struct Call {
    pub(super) span: Item,
    pub(super) selector: usize,
    arguments: Vec<Argument>,
    #[cfg(test)]
    pub(super) method: String,
    #[cfg(test)]
    pub(super) csend: bool,
}
impl Call {
    pub(super) fn items(&self, fixed: bool) -> Option<Vec<Item>> {
        let first = self.arguments.first()?;
        if self.arguments.len() < 2
            && !first
                .hash
                .as_ref()
                .is_some_and(|hash| hash.pairs.len() >= 2)
        {
            return None;
        }
        if fixed {
            let mut result: Vec<_> = self.arguments[..self.arguments.len() - 1]
                .iter()
                .map(|arg| arg.span)
                .collect();
            let last = self.arguments.last().unwrap();
            if let Some(hash) = last.hash.as_ref().filter(|hash| !hash.braces) {
                result.extend(hash.pairs.iter().copied());
            } else {
                result.push(last.span);
            }
            Some(result)
        } else if let Some(hash) = first.hash.as_ref().filter(|hash| !hash.braces) {
            Some(hash.pairs.clone())
        } else {
            Some(self.arguments.iter().map(|arg| arg.span).collect())
        }
    }
}
fn span(node: Node<'_>) -> Item {
    Item {
        start: node.start_byte(),
        end: node.end_byte(),
    }
}
fn semantic(node: Node<'_>) -> bool {
    !node.is_extra()
        && !matches!(
            node.kind(),
            "comment" | "empty_statement" | "heredoc_body" | "heredoc_end"
        )
}
#[derive(Clone, Copy)]
struct Override {
    id: usize,
    span: Item,
    non_hash: bool,
}
fn grouped(nodes: Vec<Node<'_>>, overrides: &[Override]) -> Vec<Argument> {
    shared::ruby_arguments::from_nodes(nodes.into_iter().filter(|node| semantic(*node)))
        .into_iter()
        .map(|arg| {
            let alias = (arg.nodes.len() == 1)
                .then(|| overrides.iter().find(|alias| alias.id == arg.nodes[0].id()))
                .flatten();
            let hash = (arg.hash && !alias.is_some_and(|alias| alias.non_hash)).then(|| Hash {
                braces: arg.braces,
                pairs: arg
                    .parts()
                    .into_iter()
                    .filter(|node| node.kind() == "pair")
                    .map(span)
                    .collect(),
            });
            Argument {
                span: alias.map_or(
                    Item {
                        start: arg.start(),
                        end: arg.end(),
                    },
                    |alias| alias.span,
                ),
                hash,
            }
        })
        .collect()
}
fn ordinary_arguments<'a>(
    mut nodes: Vec<Node<'a>>,
    projections: &[UnaryCommandCall<'a>],
) -> Vec<Argument> {
    nodes.retain(|node| semantic(*node));
    let mut overrides = Vec::new();
    if let (Some(first), Some(last)) = (nodes.first().copied(), nodes.last().copied()) {
        if let Some(command) = projections
            .iter()
            .filter(|call| {
                call.selector.start_byte() >= first.start_byte()
                    && call.selector.start_byte() < first.end_byte()
                    && call.end > first.end_byte()
                    && call.end <= last.end_byte()
            })
            .max_by_key(|call| call.end)
        {
            overrides.push(Override {
                id: first.id(),
                span: Item {
                    start: first.start_byte(),
                    end: command.end,
                },
                non_hash: true,
            });
            nodes.retain(|node| node.id() == first.id() || node.start_byte() >= command.end);
        }
    }
    grouped(nodes, &overrides)
}
fn csend(source: &SourceFile, node: Node<'_>) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| node_bytes(source, operator) == b"&.")
}
fn normal<'a>(
    source: &SourceFile,
    node: Node<'a>,
    projections: &[UnaryCommandCall<'a>],
) -> Option<Call> {
    if node.has_error() {
        return None;
    }
    let method_node = node.child_by_field_name("method");
    if method_node.is_some_and(|method| matches!(method.kind(), "super" | "yield")) {
        return None;
    }
    let safe = csend(source, node);
    let mut method = method_node.map_or_else(
        || "call".into(),
        |method| String::from_utf8_lossy(node_bytes(source, method)).into_owned(),
    );
    let mut nodes = argument_nodes(node);
    let mut range = span(node);
    if shared::is_setter_call(node) {
        if !method.ends_with('=') {
            method.push('=');
        }
        if let Some(assignment) = node.parent().filter(|parent| parent.kind() == "assignment") {
            if let Some(value) = assignment.child_by_field_name("right") {
                nodes.push(value);
                range.end = assignment.end_byte();
            }
        }
    }
    if method == "[]=" && !safe {
        return None;
    }
    let selector = method_node.map(|method| method.start_byte()).or_else(|| {
        node.child_by_field_name("arguments")
            .map(|args| args.start_byte())
    })?;
    Some(Call {
        span: range,
        selector,
        arguments: ordinary_arguments(nodes, projections),
        #[cfg(test)]
        method,
        #[cfg(test)]
        csend: safe,
    })
}
fn index<'a>(node: Node<'a>, projections: &[UnaryCommandCall<'a>]) -> Option<Call> {
    if node.has_error() || shared::is_setter_call(node) {
        return None;
    }
    let object = node.child_by_field_name("object")?;
    let nodes = named_kids(node)
        .into_iter()
        .filter(|child| *child != object)
        .collect();
    let mut cursor = node.walk();
    let selector = node
        .children(&mut cursor)
        .find(|child| child.kind() == "[")?
        .start_byte();
    Some(Call {
        span: span(node),
        selector,
        arguments: ordinary_arguments(nodes, projections),
        #[cfg(test)]
        method: "[]".into(),
        #[cfg(test)]
        csend: false,
    })
}
fn string_parts(node: Node<'_>, source: &SourceFile) -> bool {
    if node_bytes(source, node) == b"__FILE__" {
        return true;
    }
    match node.kind() {
        "interpolation" | "parenthesized_statements" | "body_statement" | "block_body" => {
            named_kids(node)
                .into_iter()
                .filter(|child| semantic(*child))
                .all(|child| string_parts(child, source))
        }
        "string" | "character" | "heredoc_beginning" => {
            ported::ruby_string_parts(source, node).is_some_and(|parts| parts.len() == 1)
        }
        _ => false,
    }
}
fn static_regexp(source: &SourceFile, node: Node<'_>, version: f64) -> bool {
    if node.kind() != "regex" {
        return false;
    }
    named_kids(node)
        .into_iter()
        .filter(|child| child.kind() == "interpolation")
        .all(|child| version < 3.3 && string_parts(child, source))
}
fn binary(source: &SourceFile, node: Node<'_>, version: f64) -> Option<Call> {
    if node.has_error() || shared::command_unary_plus(source, node).is_some() {
        return None;
    }
    let operator = node.child_by_field_name("operator")?;
    if matches!(node_bytes(source, operator), b"and" | b"or" | b"&&" | b"||") {
        return None;
    }
    let left = node.child_by_field_name("left")?;
    if node_bytes(source, operator) == b"=~" && static_regexp(source, left, version) {
        return None;
    }
    let right = node.child_by_field_name("right")?;
    Some(Call {
        span: span(node),
        selector: operator.start_byte(),
        arguments: grouped(vec![right], &[]),
        #[cfg(test)]
        method: String::from_utf8_lossy(node_bytes(source, operator)).into_owned(),
        #[cfg(test)]
        csend: false,
    })
}
fn projected(source: &SourceFile, command: &UnaryCommandCall<'_>) -> Option<Call> {
    let method = String::from_utf8_lossy(node_bytes(source, command.selector)).into_owned();
    let safe = command
        .selector
        .parent()
        .is_some_and(|parent| parent.kind() == "call" && csend(source, parent));
    if method == "[]=" && !safe {
        return None;
    }
    let overrides: Vec<_> = command
        .arguments
        .iter()
        .map(|arg| Override {
            id: arg.node.id(),
            span: Item {
                start: arg.start,
                end: arg.end,
            },
            non_hash: arg.unary_plus,
        })
        .collect();
    let arguments = grouped(
        command.arguments.iter().map(|arg| arg.node).collect(),
        &overrides,
    );
    Some(Call {
        span: Item {
            start: command.start,
            end: command.end,
        },
        selector: command.selector.start_byte(),
        arguments,
        #[cfg(test)]
        method,
        #[cfg(test)]
        csend: safe,
    })
}
pub(super) fn collect(source: &SourceFile, nodes: &[Node<'_>], config: &CopConfig) -> Vec<Call> {
    let projections: Vec<_> = nodes
        .iter()
        .copied()
        .filter_map(|node| shared::command_unary_plus_call(source, node))
        .collect();
    let version = config.get_f64(
        "TargetRubyVersion",
        config
            .get_str("TargetRubyVersion", "3.4")
            .parse()
            .unwrap_or(3.4),
    );
    let mut calls: Vec<_> = projections
        .iter()
        .filter_map(|call| projected(source, call))
        .collect();
    for node in nodes.iter().copied() {
        let call = match node.kind() {
            "call" => normal(source, node, &projections),
            "element_reference" => index(node, &projections),
            "binary" => binary(source, node, version),
            _ => None,
        };
        if let Some(call) = call {
            calls.push(call);
        }
    }
    calls.sort_by_key(|call| (call.span.start, std::cmp::Reverse(call.span.end)));
    calls
}
