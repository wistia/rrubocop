use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

use crate::cop::rspec::subject_scopes::{self, Names};
fn matcher(source: &SourceFile, node: Node<'_>) -> bool {
    let mut pending = vec![node];
    while let Some(child) = pending.pop() {
        if !safe_send(source, child)
            && call_receiver(child).is_none()
            && matches!(
                name(source, child),
                Some(
                    b"receive" | b"receive_messages" | b"receive_message_chain" | b"have_received"
                )
            )
        {
            return true;
        }
        pending.extend(code_children(child));
    }
    false
}
fn expectation(source: &SourceFile, node: Node<'_>, names: &[Vec<u8>]) -> bool {
    if !runner(source, node) {
        return false;
    }
    let args = call_args(node);
    if args.len() != 1 || !matcher(source, args[0]) {
        return false;
    }
    let Some(receiver) = call_receiver(node) else {
        return false;
    };
    if safe_send(source, receiver)
        || call_receiver(receiver).is_some()
        || call_block(receiver).is_some()
    {
        return false;
    }
    if bare(source, receiver, b"is_expected") && call_args(receiver).is_empty() {
        return true;
    }
    if !matches!(name(source, receiver), Some(b"expect" | b"allow")) {
        return false;
    }
    let targets = call_args(receiver);
    if targets.len() != 1
        || call_receiver(targets[0]).is_some()
        || safe_send(source, targets[0])
        || call_block(targets[0]).is_some()
        || !call_args(targets[0]).is_empty()
    {
        return false;
    }
    name(source, targets[0])
        .is_some_and(|name| name == b"subject" || names.iter().any(|subject| subject == name))
}
fn walk(
    cop: &SubjectStub,
    source: &SourceFile,
    node: Node<'_>,
    subjects: &Names,
    overrides: &Names,
    inherited: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = inherited.to_vec();
    names.extend(
        subjects
            .get(&super::cache::NodeRange::new(node))
            .into_iter()
            .flatten()
            .cloned(),
    );
    if let Some(overrides) = overrides.get(&super::cache::NodeRange::new(node)) {
        names.retain(|name| !overrides.contains(name));
    }
    let children = if expectation(source, node, &names) {
        report(
            cop,
            source,
            node.start_byte(),
            header_end(node),
            "Do not stub methods of the object under test.".into(),
            diagnostics,
        );
        if call_block(node).is_none() {
            return;
        }
        subject_scopes::body_children(source, node)
    } else {
        subject_scopes::children(source, node)
    };
    for child in children {
        walk(cop, source, child, subjects, overrides, &names, diagnostics);
    }
}

pub struct SubjectStub;
impl Cop for SubjectStub {
    fn name(&self) -> &'static str {
        "RSpec/SubjectStub"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["program"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        for top in top_groups(source, node) {
            let subjects = crate::cop::rspec::subject_scopes::definitions(source, top, false);
            let overrides = crate::cop::rspec::subject_scopes::definitions(source, top, true);
            walk(self, source, top, &subjects, &overrides, &[], diagnostics);
        }
    }
}
