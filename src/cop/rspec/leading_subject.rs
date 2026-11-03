use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::separation::numbered_block;
use crate::cop::rspec::{ordering, parity_helpers::*};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct LeadingSubject;
impl Cop for LeadingSubject {
    fn name(&self) -> &'static str {
        "RSpec/LeadingSubject"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_block(node).is_none()
            || !ordering::helper(source, node, &[b"subject", b"subject!"])
            || !inside_root_spec_group(source, node)
        {
            return;
        }
        let mut ancestor = node.parent();
        let mut parent = None;
        while let Some(candidate) = ancestor {
            if call_block(candidate).is_some() && !numbered_block(source, candidate) {
                parent = Some(candidate);
                break;
            }
            ancestor = candidate.parent();
        }
        let Some(parent) = parent else {
            return;
        };
        let siblings = ordering::body(parent);
        if siblings.len() < 2 {
            return;
        }
        let Some(index) = siblings.iter().position(|sibling| *sibling == node) else {
            return;
        };
        let previous = &siblings[..index];
        let Some(offender) = previous.iter().copied().find(|sibling| {
            ordering::helper(source, *sibling, &[b"let", b"let!"])
                || ordering::hook(source, *sibling)
                || ordering::example(source, *sibling)
                || ordering::group(source, *sibling)
                || ordering::includes(source, *sibling)
        }) else {
            return;
        };
        let diagnostic = report(
            self,
            source,
            node.start_byte(),
            node.end_byte(),
            format!(
                "Declare `subject` above any other `{}` declarations.",
                String::from_utf8_lossy(name(source, offender).unwrap())
            ),
            diagnostics,
        );
        if corrections.is_none() {
            return;
        }
        let offender_index = previous
            .iter()
            .position(|sibling| *sibling == offender)
            .unwrap();
        let crossed = &previous[offender_index..];
        let moved_name = ordering::definition_name(source, node);
        if name(source, node) == Some(b"subject!")
            || !ordering::stable_arguments(source, node)
            || crossed.iter().any(|sibling| {
                ordering::includes(source, *sibling)
                    || ordering::helper(
                        source,
                        *sibling,
                        &[b"let", b"let!", b"subject", b"subject!"],
                    ) && (ordering::definition_name(source, *sibling) == moved_name
                        || ordering::definition_name(source, *sibling).as_deref()
                            == Some("subject")
                        || matches!(name(source, *sibling), Some(b"subject" | b"subject!")))
            })
        {
            return;
        }
        if ordering::move_node(source, node, offender, false, &mut corrections, self.name()) {
            diagnostics[diagnostic].corrected = true;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LeadingSubject, "cops/rspec/leading_subject");
}
