//! Style/Attr — prefer attr_reader / attr_accessor over bare `attr`.

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Attr;

impl Cop for Attr {
    fn name(&self) -> &'static str {
        "Style/Attr"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"attr"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"attr") || call_receiver(node).is_some() {
            return;
        }
        let args = attr_args(source, node);
        if args.is_empty() {
            return;
        }
        report(self, source, node, &args, diagnostics, &mut corrections);
    }
}

fn attr_args<'a>(_source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    let args = argument_nodes(node);
    if args.is_empty() {
        command_args(node)
    } else {
        args
    }
}

fn command_args(node: Node<'_>) -> Vec<Node<'_>> {
    let meth_id = node.child_by_field_name("method").map(|m| m.id());
    let mut cur = node.walk();
    let mut out: Vec<_> = node
        .named_children(&mut cur)
        .filter(|ch| meth_id != Some(ch.id()) && ch.kind() != "block")
        .collect();
    if out.is_empty() {
        out = named_after_method(node);
    }
    out
}

fn named_after_method(node: Node<'_>) -> Vec<Node<'_>> {
    let meth_end = node
        .child_by_field_name("method")
        .map(|m| m.start_byte())
        .unwrap_or(0);
    let mut cur = node.walk();
    node.children(&mut cur)
        .filter(|ch| ch.is_named() && ch.kind() != "block" && ch.start_byte() > meth_end)
        .collect()
}

fn report(
    cop: &Attr,
    source: &SourceFile,
    node: Node<'_>,
    args: &[Node<'_>],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (replacement, has_boolean_last) = attr_replacement(source, args);
    let meth = method_node(node);
    let (line, col) = source.offset_to_line_col(meth.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        format!("Do not use `attr`. Use `{replacement}` instead."),
    );
    apply_corr(
        cop,
        source,
        node,
        args,
        meth,
        replacement,
        has_boolean_last,
        corrections,
        &mut diag,
    );
    diagnostics.push(diag);
}

fn attr_replacement(source: &SourceFile, args: &[Node<'_>]) -> (&'static str, bool) {
    let last = args.last().copied();
    let has_true = last.is_some_and(|a| node_bytes(source, a) == b"true");
    let has_false = last.is_some_and(|a| node_bytes(source, a) == b"false");
    let replacement = if has_true {
        "attr_accessor"
    } else {
        "attr_reader"
    };
    (replacement, has_true || has_false)
}

fn method_node(node: Node<'_>) -> Node<'_> {
    node.child_by_field_name("method")
        .or_else(|| node.child_by_field_name("name"))
        .unwrap_or(node)
}

fn apply_corr(
    cop: &Attr,
    _source: &SourceFile,
    node: Node<'_>,
    args: &[Node<'_>],
    meth: Node<'_>,
    replacement: &str,
    has_boolean_last: bool,
    corrections: &mut Option<&mut Vec<Correction>>,
    diag: &mut Diagnostic,
) {
    let Some(corr) = corrections else {
        return;
    };
    corr.push(Correction {
        start: meth.start_byte(),
        end: meth.end_byte(),
        replacement: replacement.to_string(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    if has_boolean_last {
        corr.push(Correction {
            start: args[0].end_byte(),
            end: closing_paren_start(node).unwrap_or(node.end_byte()),
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
    }
    diag.corrected = true;
}

fn closing_paren_start(node: Node<'_>) -> Option<usize> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|ch| !ch.is_named() && ch.kind() == ")")
        .map(|ch| ch.start_byte())
}
