use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block, is_example};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn spy_assignment<'a>(
    source: &SourceFile,
    assignment: Node<'a>,
) -> Option<(Node<'a>, Node<'a>, Node<'a>)> {
    if assignment.kind() != "assignment" {
        return None;
    }
    let left = assignment
        .child_by_field_name("left")
        .filter(|left| left.kind() == "identifier")?;
    let right = assignment.child_by_field_name("right")?;
    if safe_send(source, right)
        || name(source, right) != Some(b"as_null_object")
        || !call_args(right).is_empty()
        || call_block(right).is_some()
    {
        return None;
    }
    let double = call_receiver(right)?;
    if !bare(source, double, b"instance_double")
        || safe_send(source, double)
        || call_block(double).is_some()
    {
        return None;
    }
    Some((left, right, double))
}

pub struct InstanceSpy;
impl Cop for InstanceSpy {
    fn name(&self) -> &'static str {
        "RSpec/InstanceSpy"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = config;
        if safe_send(source, node)
            || !bare_rspec_call(source, node).is_some_and(is_example)
            || call_block(node).is_none()
            || numbered_block(source, node)
        {
            return;
        }
        let mut descendants = Vec::new();
        let mut pending = code_children(node);
        while let Some(child) = pending.pop() {
            descendants.push(child);
            pending.extend(code_children(child));
        }
        let assignments: Vec<_> = descendants
            .iter()
            .filter_map(|child| spy_assignment(source, *child))
            .collect();
        if assignments.is_empty() {
            return;
        }
        let received: std::collections::HashSet<Vec<u8>> = descendants
            .iter()
            .filter_map(|child| {
                if safe_send(source, *child) || name(source, *child) != Some(b"to") {
                    return None;
                }
                let expectation = call_receiver(*child)?;
                if !bare(source, expectation, b"expect") {
                    return None;
                }
                let expected = call_args(expectation);
                let matchers = call_args(*child);
                (expected.len() == 1
                    && expected[0].kind() == "identifier"
                    && crate::cop::ported::local_identifier(source, expected[0])
                    && !matchers.is_empty()
                    && bare(source, matchers[0], b"have_received")
                    && call_block(matchers[0]).is_none())
                .then(|| node_bytes(source, expected[0]).to_vec())
            })
            .collect();
        for (left, right, double) in assignments {
            let used = received.contains(node_bytes(source, left));
            if !used {
                continue;
            }
            let index = report(
                self,
                source,
                double.start_byte(),
                header_end(double),
                "Use `instance_spy` when you check your double with `have_received`.".into(),
                diagnostics,
            );
            if corrections.is_none() {
                continue;
            }
            let target = selector(double);
            let mut replacement =
                String::from_utf8_lossy(&source.as_bytes()[double.start_byte()..header_end(right)])
                    .into_owned();
            let dot = right.child_by_field_name("operator").unwrap();
            replacement.replace_range(
                dot.start_byte() - double.start_byte()
                    ..selector(right).end_byte() - double.start_byte(),
                "",
            );
            replacement.replace_range(
                target.start_byte() - double.start_byte()..target.end_byte() - double.start_byte(),
                "instance_spy",
            );
            if preserving_replace(
                source,
                right,
                &mut corrections,
                double.start_byte(),
                header_end(right),
                replacement,
                self.name(),
            ) {
                diagnostics[index].corrected = true;
            }
        }
    }
}
