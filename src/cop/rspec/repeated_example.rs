use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn same_body(source: &SourceFile, left: Node<'_>, right: Node<'_>) -> bool {
    match (
        call_block(left).and_then(|block| block.child_by_field_name("body")),
        call_block(right).and_then(|block| block.child_by_field_name("body")),
    ) {
        (None, None) => true,
        (Some(a), Some(b)) => eql_expression(source, a, b),
        _ => false,
    }
}

pub struct RepeatedExample;
impl Cop for RepeatedExample {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedExample"
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
        let its: Vec<_> = examples
            .iter()
            .map(|example| name(source, *example) == Some(b"its"))
            .collect();
        let repeats = repeated_flags(examples.len(), |left, right| {
            its[left] == its[right]
                && arguments[left].is_empty() == arguments[right].is_empty()
                && if its[left] {
                    eql_arguments(source, &arguments[left], &arguments[right])
                } else {
                    eql_arguments(
                        source,
                        arguments[left].get(1..).unwrap_or_default(),
                        arguments[right].get(1..).unwrap_or_default(),
                    )
                }
                && same_body(source, examples[left], examples[right])
        });
        for (position, example) in examples.iter().enumerate() {
            let repeated = repeats[position];
            if repeated {
                report(
                    self,
                    source,
                    example.start_byte(),
                    example.end_byte(),
                    "Don't repeat examples within an example group.".into(),
                    diagnostics,
                );
            }
        }
    }
}
