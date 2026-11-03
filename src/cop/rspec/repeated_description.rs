use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct RepeatedDescription;
impl Cop for RepeatedDescription {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedDescription"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        crate::cop::rspec::helpers::example_and_group_names()
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if numbered_block(source, node)
            || !bare_rspec_call(source, node).is_some_and(is_example_group)
            || call_block(node).is_none()
        {
            return;
        }
        let examples = scoped_examples(source, node);
        let arguments: Vec<_> = examples.iter().map(|example| call_args(*example)).collect();
        let kinds: Vec<_> = examples
            .iter()
            .map(|example| name(source, *example) == Some(b"its"))
            .collect();
        let repeats = repeated_flags(examples.len(), |left, right| {
            !arguments[left].is_empty()
                && !arguments[right].is_empty()
                && kinds[left] == kinds[right]
                && if kinds[left] {
                    eql_expression(source, examples[left], examples[right])
                } else {
                    eql_arguments(source, &arguments[left], &arguments[right])
                }
        });
        for (position, example) in examples.iter().enumerate() {
            let its = kinds[position];
            let repeated = repeats[position];
            if repeated {
                report(
                    self,
                    source,
                    example.start_byte(),
                    if its {
                        example.end_byte()
                    } else {
                        header_end(*example)
                    },
                    "Don't repeat descriptions within an example group.".into(),
                    diagnostics,
                );
            }
        }
    }
}
