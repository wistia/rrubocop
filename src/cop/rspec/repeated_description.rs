use crate::cop::rspec::helpers::{
    RSPEC_INCLUDE, bare_rspec_call, call_block, is_example, is_group,
};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::call_receiver;
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
        let mut examples = Vec::new();
        let mut pending = code_children(node);
        while let Some(child) = pending.pop() {
            if numbered_block(source, child) {
                pending.extend(code_children(child));
                continue;
            }
            let method = bare_rspec_call(source, child);
            if call_block(child).is_some()
                && call_receiver(child).is_none()
                && method.is_some_and(is_example)
            {
                examples.push(child);
                continue;
            }
            if call_block(child).is_some()
                && (method.is_some_and(is_group)
                    || call_receiver(child).is_none()
                        && matches!(
                            name(source, child),
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
            pending.extend(code_children(child));
        }
        examples.sort_by_key(Node::start_byte);
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
                        same_expression(source, *example, *other)
                    } else {
                        same_arguments(source, &args, &call_args(*other))
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
