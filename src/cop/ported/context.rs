use crate::cop::{CopConfig, shared};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) struct Context<'a> {
    pub source: &'a SourceFile,
    pub config: &'a CopConfig,
    render_edits: bool,
}
impl<'a> Context<'a> {
    pub fn new(source: &'a SourceFile, config: &'a CopConfig) -> Self {
        Self::with_edits(source, config, true)
    }

    pub fn with_edits(source: &'a SourceFile, config: &'a CopConfig, render_edits: bool) -> Self {
        Self {
            source,
            config,
            render_edits,
        }
    }

    pub fn render_edit(&self, render: impl FnOnce() -> String) -> String {
        if self.render_edits && !self.config.autocorrect_disabled() {
            render()
        } else {
            String::new()
        }
    }

    pub fn text(&self, node: Node<'_>) -> &'a str {
        std::str::from_utf8(shared::node_bytes(self.source, node)).unwrap_or("")
    }
    pub fn method(&self, node: Node<'_>) -> &'a str {
        if node.kind() == "identifier" {
            return self.text(node);
        }
        shared::call_method_name(self.source, node)
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("")
    }
    pub fn value(&self, node: Node<'_>) -> &'a str {
        self.text(node)
            .trim_start_matches(':')
            .trim_matches(['\'', '"'])
    }
    pub fn receiver(&self, node: Node<'_>) -> &'a str {
        node.child_by_field_name("receiver")
            .map(|n| self.text(n))
            .unwrap_or("")
    }
    pub fn symbol(&self, node: Node<'_>) -> bool {
        matches!(
            node.kind(),
            "simple_symbol" | "delimited_symbol" | "hash_key_symbol"
        )
    }
    pub fn static_symbol(&self, node: Node<'_>) -> Option<std::borrow::Cow<'a, str>> {
        symbols::value(self, node)
    }
    pub fn constant_name(&self, node: Node<'_>) -> Option<std::borrow::Cow<'a, str>> {
        constants::name(self, node)
    }
    pub fn static_string(&self, node: Node<'_>) -> Option<std::borrow::Cow<'a, str>> {
        symbols::string(self, node)
    }
    pub fn ruby_static_string(&self, node: Node<'_>) -> Option<std::borrow::Cow<'a, str>> {
        symbols::ruby_string(self, node)
    }
    pub fn ruby_string_parts(&self, node: Node<'_>) -> Option<Vec<String>> {
        symbols::ruby_parts(self, node)
    }
    pub fn ruby_heredoc_span(&self, node: Node<'_>) -> Option<(usize, usize)> {
        symbols::heredoc_span(self, node)
    }
    pub fn duplicate_literal_bytes(&self, node: Node<'_>) -> Option<(Vec<u8>, bool)> {
        symbols::duplicate_bytes(self, node)
    }
    pub fn static_value(&self, node: Node<'_>) -> Option<std::borrow::Cow<'a, str>> {
        self.static_symbol(node)
            .or_else(|| self.static_string(node))
    }
    pub fn static_pair_value<'n>(&self, node: Node<'n>, key: &str) -> Option<Node<'n>> {
        option_pairs(node).find_map(|pair| {
            let name = field(pair, "key")?;
            (self.static_value(name).as_deref() == Some(key))
                .then(|| field(pair, "value"))
                .flatten()
        })
    }
    pub fn pair_value<'n>(&self, node: Node<'n>, key: &str) -> Option<Node<'n>> {
        option_pairs(node).find_map(|p| {
            let k = p.child_by_field_name("key")?;
            (self.value(k) == key)
                .then(|| p.child_by_field_name("value"))
                .flatten()
        })
    }
}
pub(super) fn args(node: Node<'_>) -> Vec<Node<'_>> {
    shared::argument_nodes(node)
}
pub(super) fn semantic_args(node: Node<'_>) -> Vec<Node<'_>> {
    args(node)
        .into_iter()
        .filter(|argument| argument.kind() != "comment")
        .collect()
}
pub(super) fn kids(node: Node<'_>) -> Vec<Node<'_>> {
    shared::named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect()
}
pub(super) fn is_call(node: Node<'_>) -> bool {
    matches!(node.kind(), "call" | "command" | "command_call")
}
pub(super) fn block(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("block")
}
pub(super) fn method_node(node: Node<'_>) -> Node<'_> {
    shared::method_node(node).unwrap_or(node)
}
pub(super) fn body(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("body")
}
pub(super) fn safe(c: &Context<'_>, node: Node<'_>) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|n| c.text(n) == "&.")
}
pub(super) fn root_constant(c: &Context<'_>, node: Node<'_>, name: &str) -> bool {
    c.text(node).trim_start_matches("::") == name
}
pub(super) fn statements(node: Node<'_>) -> Vec<Node<'_>> {
    if matches!(
        node.kind(),
        "program" | "body_statement" | "block_body" | "then" | "else"
    ) {
        kids(node)
    } else {
        vec![node]
    }
}
pub(super) fn canonical(node: Node<'_>, c: &Context<'_>) -> String {
    let mut result = String::new();
    append_canonical(node, c, &mut result);
    result
}

fn append_canonical(node: Node<'_>, c: &Context<'_>, result: &mut String) {
    if node.named_child_count() == 0 {
        result.push_str(node.kind());
        result.push(':');
        result.push_str(c.text(node));
        return;
    }
    let mut cursor = node.walk();
    let mut first = true;
    for child in node
        .children(&mut cursor)
        .filter(|child| child.kind() != "comment")
    {
        if !first {
            result.push('|');
        }
        first = false;
        append_canonical(child, c, result);
    }
}
pub(super) fn operator<'a>(c: &'a Context<'_>, n: Node<'_>) -> &'a str {
    n.child_by_field_name("operator")
        .map(|o| c.text(o))
        .unwrap_or("")
}
pub(super) fn send_end(n: Node<'_>) -> usize {
    block(n).map(|b| b.start_byte()).unwrap_or(n.end_byte())
}
pub(super) fn field<'n>(n: Node<'n>, name: &str) -> Option<Node<'n>> {
    n.child_by_field_name(name)
}
pub(super) fn literal(n: Node<'_>) -> bool {
    matches!(
        n.kind(),
        "integer"
            | "float"
            | "string"
            | "simple_symbol"
            | "delimited_symbol"
            | "nil"
            | "true"
            | "false"
            | "array"
            | "hash"
            | "regex"
    )
}

mod options;
pub(super) use options::option_pairs;

mod bindings;
pub(super) use bindings::local_identifier;

#[cfg(test)]
mod tests;

mod finding;
pub(super) use finding::Finding;
mod walk;
pub(super) use walk::{ancestors, descendants};

mod symbols;

mod constants;

#[cfg(test)]
mod literal_tests;

pub(super) fn heredoc_end(c: &Context<'_>, n: Node<'_>) -> Option<usize> {
    symbols::heredoc_end(c, n)
}
