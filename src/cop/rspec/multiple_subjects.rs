use crate::cop::rspec::helpers::{
    RSPEC_INCLUDE, bare_rspec_call, call_block, is_example, is_group,
};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn subjects<'a>(source: &SourceFile, group: Node<'a>) -> Vec<Node<'a>> {
    let mut result = Vec::new();
    let mut pending = code_children(group);
    while let Some(node) = pending.pop() {
        if call_block(node).is_some() && !numbered_block(source, node) {
            let method = bare_rspec_call(source, node);
            if call_receiver(node).is_none() && matches!(method, Some(b"subject" | b"subject!")) {
                result.push(node);
                continue;
            }
            if method.is_some_and(is_group)
                || call_receiver(node).is_none()
                    && (method.is_some_and(is_example)
                        || matches!(
                            method,
                            Some(
                                b"include_examples"
                                    | b"include_context"
                                    | b"it_behaves_like"
                                    | b"it_should_behave_like"
                            )
                        ))
            {
                continue;
            }
        }
        pending.extend(code_children(node));
    }
    result.sort_by_key(Node::start_byte);
    result
}

pub struct MultipleSubjects;
impl Cop for MultipleSubjects {
    fn name(&self) -> &'static str {
        "RSpec/MultipleSubjects"
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
        if numbered_block(source, node)
            || !bare_rspec_call(source, node).is_some_and(is_example_group)
            || call_block(node).is_none()
        {
            return;
        }
        let subjects = subjects(source, node);
        for subject in subjects
            .iter()
            .take(subjects.len().saturating_sub(1))
            .copied()
        {
            let index = report(
                self,
                source,
                subject.start_byte(),
                subject.end_byte(),
                "Do not set more than one subject per example group".into(),
                diagnostics,
            );
            if name(source, subject) != Some(b"subject") {
                diagnostics[index].correctable = false;
                continue;
            }
            let corrected = if !call_args(subject).is_empty() {
                let selector = subject.child_by_field_name("method").unwrap();
                preserving_replace(
                    source,
                    subject,
                    &mut corrections,
                    selector.start_byte(),
                    selector.end_byte(),
                    "let",
                    self.name(),
                )
            } else {
                let bytes = source.as_bytes();
                let mut start = subject.start_byte();
                while start > 0 && bytes[start - 1] != b'\n' {
                    start -= 1;
                }
                let mut end = subject.end_byte();
                while end < bytes.len() && bytes[end] != b'\n' {
                    end += 1;
                }
                if end < bytes.len() {
                    end += 1;
                }
                // Whole-line deletion must not consume neighboring Ruby statements.
                if !bytes[start..subject.start_byte()]
                    .iter()
                    .all(u8::is_ascii_whitespace)
                    || !bytes[subject.end_byte()..end]
                        .iter()
                        .all(u8::is_ascii_whitespace)
                {
                    false
                } else {
                    preserving_replace(
                        source,
                        subject,
                        &mut corrections,
                        start,
                        end,
                        "",
                        self.name(),
                    )
                }
            };
            if corrected {
                diagnostics[index].corrected = true;
            }
        }
    }
}
