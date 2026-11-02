//! Shared tree-sitter helpers for cops (nitrocop-inspired, AST-adapted).

mod bare_args;
mod command_call;
pub use command_call::{
    CommandArgument, CommandUnaryPlus, UnaryCommandCall, command_unary_plus,
    command_unary_plus_call,
};
mod regexp;
pub use regexp::{captures_observed, decode_ruby_double_quoted_content, is_literal_regexp_text};
mod block_pass;
mod integer;
mod node_util;
mod send_range;
pub use integer::integer_value;
pub use send_range::{end as send_end, is_setter_call};
mod ruby_word;
pub use ruby_word::word_value;
pub mod ruby_expression;
pub(crate) mod ruby_literal;
pub mod ruby_semantics;

pub use bare_args::replace_arg;
pub use block_pass::{EXCLUDED_BLOCK_PARENTS, METHOD_STOPS, NESTED_SCOPES, Passes, ScanOpts, scan};

pub use node_util::{
    child_kind, end_keyword, last_child_kind, line_blank, line_indent, named_kids, node_bytes,
    node_col, node_line, node_text,
};

use tree_sitter::Node;

use crate::correction::Correction;
use crate::parse::source::SourceFile;

/// Method/name field of a call-like node.
pub fn method_node(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("method")
        .or_else(|| node.child_by_field_name("name"))
}

/// Push a byte-range replacement when collecting corrections.
pub fn push_replace(
    corrections: &mut Option<&mut Vec<Correction>>,
    start: usize,
    end: usize,
    replacement: impl Into<String>,
    cop_name: &'static str,
) -> bool {
    let Some(corr) = corrections.as_deref_mut() else {
        return false;
    };
    corr.push(Correction {
        start,
        end,
        replacement: replacement.into(),
        cop_name,
        cop_index: 0,
        owner: None,
    });
    true
}

/// True if `name` is SCREAMING_SNAKE_CASE (RuboCop Naming/ConstantName).
/// Allows Unicode uppercase letters (e.g. `KIND_НАЧИСЛЕНИЕ`).
pub fn is_screaming_snake_case(name: &[u8]) -> bool {
    let Ok(s) = std::str::from_utf8(name) else {
        return false;
    };
    if s.is_empty() {
        return false;
    }
    let mut has_letter = false;
    for c in s.chars() {
        if c.is_uppercase() {
            has_letter = true;
        } else if c.is_ascii_digit() || c == '_' {
            // ok
        } else {
            return false;
        }
    }
    has_letter
}

/// Method name of a `call` / `command` / `method` node, if any.
pub fn call_method_name<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    let meth = node
        .child_by_field_name("method")
        .or_else(|| node.child_by_field_name("name"))?;
    Some(node_bytes(source, meth))
}

/// Receiver of a call-like node.
pub fn call_receiver(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("receiver")
}

/// True if node is a constant read whose text equals `name` (e.g. `JSON`).
pub fn is_const_named(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    match node.kind() {
        "constant" => node_bytes(source, node) == name,
        "scope_resolution" => node
            .child_by_field_name("name")
            .map(|n| node_bytes(source, n) == name)
            .unwrap_or(false),
        _ => false,
    }
}

/// `FOO = …` / `Foo::BAR = …` assignment (not a plain local).
pub fn is_const_assign(node: Node<'_>) -> bool {
    if node.kind() != "assignment" {
        return false;
    }
    node.child_by_field_name("left")
        .or_else(|| node.named_child(0))
        .is_some_and(|lhs| matches!(lhs.kind(), "constant" | "scope_resolution"))
}

/// Direct named children (field `arguments` or positional).
pub fn argument_nodes<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    if let Some(args) = node.child_by_field_name("arguments") {
        let mut cur = args.walk();
        return args
            .named_children(&mut cur)
            .filter(|n| !matches!(n.kind(), "," | "comment"))
            .collect();
    }
    Vec::new()
}

/// Walk all descendants, invoking `f` on each.
pub fn for_each_descendant<'a>(node: Node<'a>, mut f: impl FnMut(Node<'a>)) {
    fn walk<'a>(node: Node<'a>, f: &mut impl FnMut(Node<'a>)) {
        f(node);
        let mut cur = node.walk();
        for child in node.children(&mut cur) {
            walk(child, f);
        }
    }
    walk(node, &mut f);
}

/// Collect all `comment` nodes under `root`.
pub fn collect_comments<'a>(root: Node<'a>) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    fn walk<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
        if node.kind() == "comment" {
            out.push(node);
        }
        let mut cur = node.walk();
        for child in node.children(&mut cur) {
            walk(child, out);
        }
    }
    walk(root, &mut out);
    out
}

/// True if bytes in [start,end) are only spaces/tabs.
pub fn only_spaces(bytes: &[u8], start: usize, end: usize) -> bool {
    if end <= start {
        return true;
    }
    bytes[start..end.min(bytes.len())]
        .iter()
        .all(|&b| b == b' ' || b == b'\t')
}

/// True if [start,end) is non-empty and contains only spaces/tabs (no newlines).
pub fn only_hspace(bytes: &[u8], start: usize, end: usize) -> bool {
    if end <= start {
        return false;
    }
    bytes[start..end.min(bytes.len())]
        .iter()
        .all(|&b| b == b' ' || b == b'\t')
}

/// True if [start,end) contains a space or tab.
pub fn has_hspace(bytes: &[u8], start: usize, end: usize) -> bool {
    if end <= start {
        return false;
    }
    bytes[start..end.min(bytes.len())]
        .iter()
        .any(|&b| b == b' ' || b == b'\t')
}

#[cfg(test)]
mod argument_tests {
    use super::*;

    fn arguments(bytes: &[u8]) -> Vec<String> {
        let source = SourceFile::from_bytes("test.rb", bytes.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        argument_nodes(tree.root_node().named_child(0).unwrap())
            .into_iter()
            .map(|node| node_text(&source, node))
            .collect()
    }

    #[test]
    fn syntax_comments_are_not_semantic_arguments() {
        assert_eq!(
            arguments(b"foo( # first\n1, # between\n2 # last\n)\n"),
            vec!["1", "2"]
        );
    }

    #[test]
    fn comment_only_parentheses_have_no_arguments() {
        assert!(arguments(b"foo( # comment\n)\n").is_empty());
    }
}
