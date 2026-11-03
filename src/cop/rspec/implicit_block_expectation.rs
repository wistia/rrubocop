use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn lambda_subject(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() == "lambda" {
        return !numbered_block(source, node);
    }
    if safe_send(source, node)
        || call_block(node).is_none()
        || numbered_block(source, node)
        || !call_args(node).is_empty()
    {
        return false;
    }
    if call_receiver(node).is_none() && matches!(name(source, node), Some(b"proc" | b"lambda")) {
        return true;
    }
    name(source, node) == Some(b"new")
        && call_receiver(node).is_some_and(|receiver| {
            receiver.kind() == "constant" && node_bytes(source, receiver) == b"Proc"
        })
}

#[derive(Clone)]
struct GroupSubject(Option<bool>);
fn group_subject(source: &SourceFile, group: Node<'_>) -> Option<bool> {
    super::cache::node_fact(source, group, || {
        let result = (|| {
            let body = call_block(group)?.child_by_field_name("body")?;
            let children = code_children(body);
            if children.len() < 2 {
                return None;
            }
            let subject = children.into_iter().find(|child| {
                !safe_send(source, *child)
                    && call_receiver(*child).is_none()
                    && matches!(name(source, *child), Some(b"subject" | b"subject!"))
                    && call_block(*child).is_some()
            })?;
            let Some(body) =
                call_block(subject).and_then(|block| block.child_by_field_name("body"))
            else {
                return Some(false);
            };
            let children = code_children(body);
            Some(children.len() == 1 && lambda_subject(source, children[0]))
        })();
        GroupSubject(result)
    })
    .0
}

pub struct ImplicitBlockExpectation;
impl Cop for ImplicitBlockExpectation {
    fn name(&self) -> &'static str {
        "RSpec/ImplicitBlockExpectation"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"is_expected", b"should", b"should_not"]
    }
    fn interested_identifier_names(&self) -> &'static [&'static [u8]] {
        self.interested_call_names()
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if safe_send(source, node)
            || call_receiver(node).is_some()
            || !matches!(
                name(source, node),
                Some(b"is_expected" | b"should" | b"should_not")
            )
        {
            return;
        }
        let mut ancestor = node.parent();
        while let Some(group) = ancestor {
            ancestor = group.parent();
            if numbered_block(source, group)
                || !bare_rspec_call(source, group).is_some_and(is_example_group)
            {
                continue;
            }
            let Some(lambda) = group_subject(source, group) else {
                continue;
            };
            if lambda {
                report(
                    self,
                    source,
                    node.start_byte(),
                    header_end(node),
                    "Avoid implicit block expectations.".into(),
                    diagnostics,
                );
            }
            return;
        }
    }
}
