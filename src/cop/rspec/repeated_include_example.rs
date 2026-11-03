use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{call_receiver, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct RepeatedIncludeExample;
impl Cop for RepeatedIncludeExample {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedIncludeExample"
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
        let items: Vec<_> = code_children(node)
            .into_iter()
            .filter(|child| {
                call_receiver(*child).is_none()
                    && !safe_send(source, *child)
                    && call_block(*child).is_none()
                    && matches!(
                        name(source, *child),
                        Some(b"include_examples" | b"it_behaves_like" | b"it_should_behave_like")
                    )
                    && call_args(*child)
                        .iter()
                        .all(|argument| literal_or_const(source, *argument))
            })
            .collect();
        let arguments: Vec<_> = items.iter().map(|item| call_args(*item)).collect();
        let repeated = repeated_indices(items.len(), |left, right| {
            eql_arguments(source, &arguments[left], &arguments[right])
        });
        for (position, item) in items.iter().enumerate() {
            let Some(first) = arguments[position].first() else {
                continue;
            };
            let repeats: Vec<_> = repeated[position]
                .iter()
                .map(|index| items[*index].start_position().row + 1)
                .collect();
            if !repeats.is_empty() {
                report(
                    self,
                    source,
                    item.start_byte(),
                    header_end(*item),
                    format!(
                        "Repeated include of shared_examples {} on line(s) {:?}",
                        node_text(source, *first),
                        repeats
                    ),
                    diagnostics,
                );
            }
        }
    }
}
