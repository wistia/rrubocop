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
            if args.is_empty() {
                continue;
            }
            let its = name(source, *example) == Some(b"its");
            let repeated = examples.iter().any(|other| {
                other != example
                    && (name(source, *other) == Some(b"its")) == its
                    && if its {
                        eql_expression(source, *example, *other)
                    } else {
                        eql_arguments(source, &args, &call_args(*other))
                    }
            });
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
