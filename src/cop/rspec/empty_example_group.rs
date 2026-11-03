use crate::cop::rspec::helpers::{
    RSPEC_INCLUDE, bare_rspec_call, call_block, is_example, is_group, is_let_helper,
};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn group(name: &[u8]) -> bool {
    is_group(name)
        && !matches!(
            name,
            b"shared_examples" | b"shared_context" | b"shared_examples_for"
        )
}
fn include(name: &[u8]) -> bool {
    matches!(
        name,
        b"it_behaves_like" | b"it_should_behave_like" | b"include_examples" | b"include_context"
    )
}
fn ordinary_block(source: &SourceFile, node: Node<'_>) -> bool {
    call_block(node).is_some() && !numbered_block(source, node)
}
fn body(node: Node<'_>) -> Vec<Node<'_>> {
    if matches!(
        node.kind(),
        "body_statement" | "body_statements" | "block_body" | "then" | "else" | "begin"
    ) {
        if code_children(node)
            .iter()
            .any(|n| matches!(n.kind(), "rescue" | "ensure"))
        {
            return vec![node];
        }
        code_children(node)
    } else {
        vec![node]
    }
}
fn block_body(node: Node<'_>) -> Vec<Node<'_>> {
    call_block(node)
        .and_then(|b| b.child_by_field_name("body"))
        .map(body)
        .unwrap_or_default()
}
fn branch_examples(source: &SourceFile, node: Node<'_>) -> bool {
    if matches!(
        node.kind(),
        "if" | "unless" | "conditional" | "if_modifier" | "unless_modifier" | "elsif"
    ) {
        ["consequence", "alternative", "body"]
            .iter()
            .filter_map(|field| node.child_by_field_name(field))
            .any(|branch| {
                if branch.kind() == "elsif" {
                    branch_examples(source, branch)
                } else {
                    examples(source, &body(branch))
                }
            })
    } else if node.kind() == "case" {
        code_children(node)
            .iter()
            .filter(|n| matches!(n.kind(), "when" | "else"))
            .any(|branch| {
                examples(
                    source,
                    &branch
                        .child_by_field_name("body")
                        .map(body)
                        .unwrap_or_else(|| body(*branch)),
                )
            })
    } else {
        false
    }
}
fn direct(source: &SourceFile, node: Node<'_>) -> bool {
    if ordinary_block(source, node) {
        if bare_rspec_call(source, node).is_some_and(|n| is_example(n) || group(n) || include(n)) {
            return true;
        }
        if call_receiver(node).is_none() && name(source, node).is_some_and(|n| hooks().contains(&n))
        {
            return false;
        }
        examples(source, &block_body(node))
    } else if call_block(node).is_none() {
        call_receiver(node).is_none()
            && name(source, node).is_some_and(|n| is_example(n) || include(n))
    } else {
        false
    }
}
fn examples(source: &SourceFile, nodes: &[Node<'_>]) -> bool {
    nodes.iter().any(|n| direct(source, *n))
        || nodes.len() > 1 && nodes.iter().any(|n| branch_examples(source, *n))
}
fn offensive(source: &SourceFile, nodes: &[Node<'_>]) -> bool {
    if nodes.len() > 1 || nodes.first().is_some_and(|n| n.kind() == "case") {
        let mut pending = nodes.to_vec();
        if nodes.len() == 1 {
            pending = code_children(nodes[0]);
        }
        while let Some(node) = pending.pop() {
            if branch_examples(source, node) {
                return false;
            }
            pending.extend(code_children(node));
        }
    }
    if nodes.len() == 1
        && matches!(
            nodes[0].kind(),
            "if" | "unless" | "conditional" | "case" | "if_modifier" | "unless_modifier"
        )
    {
        !branch_examples(source, nodes[0])
    } else {
        !examples(source, nodes)
    }
}
fn removable_body(source: &SourceFile, nodes: &[Node<'_>]) -> bool {
    nodes.iter().all(|node| {
        pure_literal(source, *node)
            || bare_rspec_call(source, *node)
                .is_some_and(|n| is_let_helper(n) || hooks().contains(&n))
                && ordinary_block(source, *node)
                && call_args(*node)
                    .iter()
                    .all(|arg| pure_literal(source, *arg))
    })
}

pub struct EmptyExampleGroup;
impl Cop for EmptyExampleGroup {
    fn name(&self) -> &'static str {
        "RSpec/EmptyExampleGroup"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !ordinary_block(source, node) || !bare_rspec_call(source, node).is_some_and(group) {
            return;
        }
        let mut ancestor = node.parent();
        while let Some(parent) = ancestor {
            if matches!(parent.kind(), "method" | "singleton_method")
                || ordinary_block(source, parent)
                    && call_receiver(parent).is_none()
                    && name(source, parent).is_some_and(is_example)
            {
                return;
            }
            ancestor = parent.parent();
        }
        let nodes = block_body(node);
        if !offensive(source, &nodes) {
            return;
        }
        let index = report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            "Empty example group detected.".into(),
            diagnostics,
        );
        if corrections.is_none()
            || !removable_body(source, &nodes)
            || !call_args(node).iter().all(|arg| {
                pure_literal(source, *arg) || matches!(arg.kind(), "constant" | "scope_resolution")
            })
        {
            return;
        }
        let bytes = source.as_bytes();
        let start = bytes[..node.start_byte()]
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |p| p + 1);
        let end = bytes[node.end_byte()..]
            .iter()
            .position(|b| *b == b'\n')
            .map_or(bytes.len(), |p| node.end_byte() + p + 1);
        if bytes[start..node.start_byte()]
            .iter()
            .any(|b| !b.is_ascii_whitespace())
            || bytes[node.end_byte()..end]
                .iter()
                .any(|b| !b.is_ascii_whitespace())
        {
            return;
        }
        if preserving_replace(source, node, &mut corrections, start, end, "", self.name()) {
            diagnostics[index].corrected = true;
        }
    }
}
