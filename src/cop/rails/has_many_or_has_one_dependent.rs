//! Preserve the original association option and immediate-parent exemptions.
use crate::cop::shared::ruby_expression::{numbered_bindings, plain_symbol};
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, method_node, named_kids, node_text,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct HasManyOrHasOneDependent;
#[derive(Clone, Copy)]
enum View<'a> {
    Node(Node<'a>),
    Block(Node<'a>),
}
impl<'a> View<'a> {
    fn node(self) -> Node<'a> {
        match self {
            Self::Node(n) | Self::Block(n) => n,
        }
    }
    fn parent(self) -> Option<Self> {
        match self {
            Self::Node(n) if n.kind() == "call" && n.child_by_field_name("block").is_some() => {
                Some(Self::Block(n))
            }
            _ => self.node().parent().and_then(normalize),
        }
    }
    fn begin(self) -> bool {
        matches!(self,Self::Node(n) if matches!(n.kind(),"program"|"body_statement"|"block_body"|"then"|"else"|"parenthesized_statements"))
    }
}
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| !matches!(n.kind(), "comment" | "empty_statement" | "heredoc_body"))
        .collect()
}
fn normalize(node: Node<'_>) -> Option<View<'_>> {
    match node.kind() {
        "argument_list" | "superclass" => node.parent().and_then(normalize),
        "program" | "body_statement" | "block_body" | "then" | "else"
            if children(node).len() <= 1 =>
        {
            node.parent().and_then(normalize)
        }
        "block" | "do_block" => node
            .parent()
            .filter(|n| n.kind() == "call")
            .map(View::Block)
            .or(Some(View::Node(node))),
        _ => Some(View::Node(node)),
    }
}
fn normal_block<'a>(source: &SourceFile, node: Node<'a>, config: &CopConfig) -> Option<Node<'a>> {
    let b = node.child_by_field_name("block")?;
    (config.get_f64("TargetRubyVersion", 3.4) < 2.7 || numbered_bindings(source, b).is_none())
        .then_some(b)
}
fn call_options(node: Node<'_>) -> (usize, Option<Vec<Node<'_>>>) {
    let args = argument_nodes(node);
    if let Some(i) = args
        .iter()
        .position(|n| matches!(n.kind(), "pair" | "hash_splat_argument"))
    {
        return (i + 1, Some(args[i..].to_vec()));
    }
    (
        args.len(),
        args.last()
            .filter(|n| n.kind() == "hash")
            .map(|n| children(*n)),
    )
}
fn valid(source: &SourceFile, options: Option<Vec<Node<'_>>>) -> bool {
    let Some(mut options) = options else {
        return false;
    };
    if let Some(first) = options
        .first()
        .filter(|n| n.kind() == "hash_splat_argument")
    {
        if let Some(hash) = children(*first).first().filter(|n| n.kind() == "hash") {
            options = children(*hash);
        }
    }
    options.iter().any(|n| {
        let Some(key) = n
            .child_by_field_name("key")
            .and_then(|n| plain_symbol(source, n))
        else {
            return false;
        };
        key == "dependent"
            || key == "through"
                && n.child_by_field_name("value")
                    .is_none_or(|v| v.kind() != "nil")
    })
}
fn with_options<'a>(
    source: &SourceFile,
    view: View<'a>,
    config: &CopConfig,
) -> Option<Vec<Node<'a>>> {
    let View::Block(node) = view else {
        return None;
    };
    if call_method_name(source, node) != Some(b"with_options") || call_receiver(node).is_some() {
        return None;
    }
    let block = normal_block(source, node, config)?;
    let params = block
        .child_by_field_name("parameters")
        .map_or_else(Vec::new, children);
    if params.len() > 1 {
        return None;
    }
    let (count, options) = call_options(node);
    (count == 1).then_some(options).flatten()
}
fn extension(source: &SourceFile, view: View<'_>, config: &CopConfig) -> bool {
    let View::Block(node) = view else {
        return false;
    };
    call_method_name(source, node) == Some(b"has_many")
        && call_receiver(node).is_none()
        && call_options(node).0 == 1
        && normal_block(source, node, config).is_some_and(|b| {
            b.child_by_field_name("parameters")
                .is_none_or(|p| children(p).is_empty())
        })
}
fn inherited(source: &SourceFile, view: View<'_>, config: &CopConfig) -> bool {
    let Some(options) = with_options(source, view, config) else {
        return false;
    };
    valid(source, Some(options))
        || view
            .parent()
            .and_then(View::parent)
            .is_some_and(|p| inherited(source, p, config))
}
fn descendants<'a>(view: View<'a>, out: &mut Vec<Node<'a>>) {
    fn walk<'a>(n: Node<'a>, out: &mut Vec<Node<'a>>, skip_block: bool) {
        out.push(n);
        for c in children(n) {
            if !skip_block || n.child_by_field_name("block") != Some(c) {
                walk(c, out, false);
            }
        }
    }
    walk(
        view.node(),
        out,
        matches!(view,View::Node(n) if n.kind()=="call"),
    );
}
fn active_resource(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() != "scope_resolution"
        || !node
            .child_by_field_name("name")
            .is_some_and(|n| node_text(source, n) == "Base")
    {
        return false;
    }
    let Some(scope) = node.child_by_field_name("scope") else {
        return false;
    };
    match scope.kind() {
        "constant" => node_text(source, scope) == "ActiveResource",
        "scope_resolution" => {
            scope.child_by_field_name("scope").is_none()
                && scope
                    .child_by_field_name("name")
                    .is_some_and(|n| node_text(source, n) == "ActiveResource")
        }
        _ => false,
    }
}
fn readonly(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "method"
        && node
            .child_by_field_name("name")
            .is_some_and(|n| node_text(source, n) == "readonly?")
        && node
            .child_by_field_name("parameters")
            .is_none_or(|n| children(n).is_empty())
        && node.child_by_field_name("body").is_some_and(|n| {
            let nodes = children(n);
            n.kind() == "true"
                || n.kind() == "body_statement" && nodes.len() == 1 && nodes[0].kind() == "true"
        })
}
fn parent_exemption(source: &SourceFile, parent: View<'_>) -> bool {
    type Key = (usize, usize, u16, bool);
    struct Exemptions(std::sync::Mutex<std::collections::HashMap<Key, bool>>);
    let node = parent.node();
    let key = (
        node.start_byte(),
        node.end_byte(),
        node.kind_id(),
        matches!(parent, View::Block(_)),
    );
    let exemptions = source.cached(0, || Exemptions(Default::default()));
    *exemptions.0.lock().unwrap().entry(key).or_insert_with(|| {
        let mut nodes = Vec::new();
        descendants(parent, &mut nodes);
        nodes
            .iter()
            .any(|n| active_resource(source, *n) || readonly(source, *n))
    })
}
impl Cop for HasManyOrHasOneDependent {
    fn name(&self) -> &'static str {
        "Rails/HasManyOrHasOneDependent"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"has_many", b"has_one"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bare = crate::cop::ported::bare_call_identifier(source, node);
        let method_name = call_method_name(source, node).or_else(|| {
            bare.then(|| source.as_bytes().get(node.start_byte()..node.end_byte()))
                .flatten()
        });
        if (!bare && !super::unknown_env::regular(node, source))
            || crate::cop::shared::is_setter_call(node)
            || !matches!(method_name, Some(b"has_many" | b"has_one"))
        {
            return;
        }
        let (count, options) = call_options(node);
        if count != 1 && valid(source, options) {
            return;
        }
        let Some(parent) = View::Node(node).parent() else {
            return;
        };
        if parent_exemption(source, parent) {
            return;
        }
        let context = if parent.begin() || extension(source, parent, config) {
            parent.parent()
        } else {
            Some(parent)
        };
        if context.is_some_and(|p| inherited(source, p, config)) {
            return;
        }
        let Some(method) = method_node(node).or_else(|| bare.then_some(node)) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(method.start_byte());
        let mut d = self.diagnostic(source, line, col, "Specify a `:dependent` option.".into());
        d.highlight_length = node_text(source, method).chars().count();
        diagnostics.push(d);
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    #[test]
    fn parent_summary_distinguishes_call_view_from_its_own_block() {
        let source = SourceFile::from_bytes(
            "app/models/example.rb",
            b"with_options dependent: :destroy do; def readonly?; true; end; end".to_vec(),
        );
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let call = children(tree.root_node())[0];
        assert!(!parent_exemption(&source, View::Node(call)));
        assert!(parent_exemption(&source, View::Block(call)));
        assert!(!parent_exemption(&source, View::Node(call)));
    }
    #[test]
    fn cached_parent_summaries_equal_the_original_descendant_predicate() {
        for text in [
            "class A < ActiveResource::Base; has_many :x; has_one :y; end",
            "class A; def readonly?; true; end; has_many :x; end",
            "class A; def readonly?; false; end; has_many :x; end",
            "with_options dependent: :destroy do; has_many :x; has_one :y; end",
            "class A; has_many :x do; has_one :y; end; end",
            "class A; has_many :x, dependent: nil; has_one :y, through: nil; end",
        ] {
            let source = SourceFile::from_bytes("app/models/example.rb", text.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            let mut pending = vec![tree.root_node()];
            while let Some(node) = pending.pop() {
                for view in [View::Node(node), View::Block(node)] {
                    let mut original = Vec::new();
                    descendants(view, &mut original);
                    let expected = original
                        .iter()
                        .any(|node| active_resource(&source, *node) || readonly(&source, *node));
                    assert_eq!(parent_exemption(&source, view), expected);
                    assert_eq!(parent_exemption(&source, view), expected);
                }
                pending.extend(children(node));
            }
        }
    }
}
