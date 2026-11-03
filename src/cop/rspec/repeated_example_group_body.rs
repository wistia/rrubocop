use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn constant<'a>(args: &[Node<'a>]) -> Option<Node<'a>> {
    args.first()
        .filter(|arg| matches!(arg.kind(), "constant" | "scope_resolution"))
        .copied()
}
fn same_group(
    source: &SourceFile,
    left: Node<'_>,
    right: Node<'_>,
    a: &[Node<'_>],
    b: &[Node<'_>],
) -> bool {
    if a.is_empty() != b.is_empty()
        || !eql_arguments(
            source,
            a.get(1..).unwrap_or_default(),
            b.get(1..).unwrap_or_default(),
        )
    {
        return false;
    }
    if !match (constant(a), constant(b)) {
        (None, None) => true,
        (Some(a), Some(b)) => eql_expression(source, a, b),
        _ => false,
    } {
        return false;
    }
    let body = |node| {
        call_block(node)
            .and_then(|block| block.child_by_field_name("body"))
            .unwrap()
    };
    eql_expression(source, body(left), body(right))
}

pub struct RepeatedExampleGroupBody;
impl Cop for RepeatedExampleGroupBody {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedExampleGroupBody"
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
                !numbered_block(source, *child)
                    && bare_rspec_call(source, *child).is_some_and(is_example_group)
                    && call_block(*child)
                        .and_then(|block| block.child_by_field_name("body"))
                        .is_some()
                    && !crate::cop::rspec::repeated_example_group_description::pending_group(
                        source, *child,
                    )
            })
            .collect();
        let arguments: Vec<_> = groups.iter().map(|group| call_args(*group)).collect();
        let repeated = repeated_indices(groups.len(), |left, right| {
            same_group(
                source,
                groups[left],
                groups[right],
                &arguments[left],
                &arguments[right],
            )
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
                        "Repeated {} block body on line(s) {:?}",
                        String::from_utf8_lossy(name(source, *group).unwrap()),
                        repeats
                    ),
                    diagnostics,
                );
            }
        }
    }
}
