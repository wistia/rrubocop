use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

use crate::cop::rspec::subject_scopes::{self, Names};
fn walk(
    cop: &RepeatedSubjectCall,
    source: &SourceFile,
    node: Node<'_>,
    subjects: &Names,
    inherited: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = inherited.to_vec();
    names.extend(subjects.get(&node.id()).into_iter().flatten().cloned());
    if subject_scopes::example(source, node) {
        let Some(body) = call_block(node).and_then(|block| block.child_by_field_name("body"))
        else {
            return;
        };
        let mut calls = Vec::new();
        let mut pending = vec![body];
        while let Some(child) = pending.pop() {
            if !safe_send(source, child)
                && call_receiver(child).is_none()
                && call_args(child).is_empty()
                && name(source, child).is_some_and(|name| {
                    name == b"subject" || names.iter().any(|subject| subject == name)
                })
            {
                calls.push(child);
            }
            pending.extend(code_children(child));
        }
        calls.sort_by_key(Node::start_byte);
        let mut seen = std::collections::HashSet::new();
        let mut reported = std::collections::HashSet::new();
        for call in calls {
            let method = name(source, call).unwrap();
            if seen.insert(method.to_vec()) || subject_scopes::parent_send(source, call) {
                continue;
            }
            let mut ancestor = call.parent();
            while let Some(parent) = ancestor {
                if name(source, parent) == Some(b"expect")
                    && call_block(parent).is_some()
                    && !numbered_block(source, parent)
                {
                    if reported.insert(parent.id()) {
                        report(
                            cop,
                            source,
                            parent.start_byte(),
                            parent.end_byte(),
                            "Calls to subject are memoized, this block is misleading".into(),
                            diagnostics,
                        );
                    }
                    break;
                }
                ancestor = parent.parent();
            }
        }
        return;
    }
    for child in subject_scopes::children(source, node) {
        walk(cop, source, child, subjects, &names, diagnostics);
    }
}

pub struct RepeatedSubjectCall;
impl Cop for RepeatedSubjectCall {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedSubjectCall"
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
            walk(self, source, top, &subjects, &[], diagnostics);
        }
    }
}
