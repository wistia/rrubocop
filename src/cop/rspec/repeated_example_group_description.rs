use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn pending_group(source: &SourceFile, node: Node<'_>) -> bool {
    if numbered_block(source, node) {
        return false;
    }
    let Some(body) = call_block(node).and_then(|block| block.child_by_field_name("body")) else {
        return false;
    };
    let children = code_children(body);
    children.len() == 1
        && call_block(children[0]).is_none()
        && call_receiver(children[0]).is_none()
        && !safe_send(source, children[0])
        && matches!(name(source, children[0]), Some(b"skip" | b"pending"))
}

pub struct RepeatedExampleGroupDescription;
impl Cop for RepeatedExampleGroupDescription {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedExampleGroupDescription"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[
            "program",
            "body_statement",
            "block_body",
            "then",
            "else",
            "when",
            "rescue",
            "ensure",
        ]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let groups: Vec<_> = code_children(node)
            .into_iter()
            .filter(|child| {
                bare_rspec_call(source, *child).is_some_and(is_example_group)
                    && call_block(*child).is_some()
                    && (numbered_block(source, *child) || !call_args(*child).is_empty())
                    && !pending_group(source, *child)
            })
            .collect();
        let arguments: Vec<_> = groups.iter().map(|group| call_args(*group)).collect();
        let numbered: Vec<_> = groups
            .iter()
            .map(|group| numbered_block(source, *group))
            .collect();
        let repeated = repeated_indices(groups.len(), |left, right| {
            match (numbered[left], numbered[right]) {
                (true, true) => true,
                (false, false) => eql_arguments(source, &arguments[left], &arguments[right]),
                _ => false,
            }
        });
        for (position, group) in groups.iter().enumerate() {
            let repeats: Vec<_> = repeated[position]
                .iter()
                .map(|index| groups[*index].start_position().row + 1)
                .collect();
            if !repeats.is_empty() {
                report(
                    self,
                    source,
                    group.start_byte(),
                    group.end_byte(),
                    format!(
                        "Repeated {} block description on line(s) {:?}",
                        String::from_utf8_lossy(name(source, *group).unwrap()),
                        repeats
                    ),
                    diagnostics,
                );
            }
        }
    }
}
