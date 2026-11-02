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
        (Some(a), Some(b)) => same_expression(source, a, b),
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
        for example in &examples {
            let args = call_args(*example);
            let its = name(source, *example) == Some(b"its");
            let repeated = examples.iter().any(|other| {
                let other_args = call_args(*other);
                other != example
                    && (name(source, *other) == Some(b"its")) == its
                    && args.is_empty() == other_args.is_empty()
                    && if its {
                        same_arguments(source, &args, &other_args)
                    } else {
                        same_arguments(
                            source,
                            args.get(1..).unwrap_or_default(),
                            other_args.get(1..).unwrap_or_default(),
                        )
                    }
                    && same_body(source, *example, *other)
            });
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
