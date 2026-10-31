use crate::cop::{CopConfig, shared};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) struct Context<'a> {
    pub source: &'a SourceFile,
    pub config: &'a CopConfig,
}
pub(super) struct Finding {
    pub start: usize,
    pub end: usize,
    pub message: String,
    pub edit: Option<(usize, usize, String)>,
    pub additional: Vec<(usize, usize, String)>,
}
impl Finding {
    pub fn new(node: Node<'_>, message: impl Into<String>) -> Self {
        Self {
            start: node.start_byte(),
            end: node.end_byte(),
            message: message.into(),
            edit: None,
            additional: vec![],
        }
    }
    pub fn range(start: usize, end: usize, message: impl Into<String>) -> Self {
        Self {
            start,
            end,
            message: message.into(),
            edit: None,
            additional: vec![],
        }
    }
    pub fn replace(mut self, node: Node<'_>, replacement: impl Into<String>) -> Self {
        self.edit = Some((node.start_byte(), node.end_byte(), replacement.into()));
        self
    }
    pub fn also_edit(mut self, start: usize, end: usize, replacement: impl Into<String>) -> Self {
        self.additional.push((start, end, replacement.into()));
        self
    }

    pub fn edit(mut self, start: usize, end: usize, replacement: impl Into<String>) -> Self {
        self.edit = Some((start, end, replacement.into()));
        self
    }
}
impl<'a> Context<'a> {
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
    pub fn symbol_pair_value<'n>(&self, node: Node<'n>, key: &str) -> Option<Node<'n>> {
        option_pairs(node).into_iter().find_map(|pair| {
            let name = field(pair, "key")?;
            (self.symbol(name) && self.value(name) == key)
                .then(|| field(pair, "value"))
                .flatten()
        })
    }
    pub fn pair_value<'n>(&self, node: Node<'n>, key: &str) -> Option<Node<'n>> {
        option_pairs(node).into_iter().find_map(|p| {
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
pub(super) fn kids(node: Node<'_>) -> Vec<Node<'_>> {
    shared::named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect()
}
pub(super) fn descendants(node: Node<'_>) -> Vec<Node<'_>> {
    let mut out = vec![node];
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        out.extend(descendants(child));
    }
    out
}
pub(super) fn ancestors(node: Node<'_>) -> Vec<Node<'_>> {
    let mut out = Vec::new();
    let mut parent = node.parent();
    while let Some(n) = parent {
        out.push(n);
        parent = n.parent();
    }
    out
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
    if node.named_child_count() == 0 {
        return format!("{}:{}", node.kind(), c.text(node));
    }
    let mut cur = node.walk();
    node.children(&mut cur)
        .filter(|n| n.kind() != "comment")
        .map(|n| canonical(n, c))
        .collect::<Vec<_>>()
        .join("|")
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

pub(super) fn option_pairs(node: Node<'_>) -> Vec<Node<'_>> {
    match node.kind() {
        "pair" => vec![node],
        "hash" => kids(node)
            .into_iter()
            .filter(|p| p.kind() == "pair")
            .collect(),
        "call" => args(node)
            .into_iter()
            .filter(|p| matches!(p.kind(), "hash" | "pair"))
            .flat_map(option_pairs)
            .collect(),
        _ => vec![],
    }
}

mod bindings;
pub(super) use bindings::local_identifier;
