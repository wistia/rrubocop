use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn literal(source: &SourceFile, node: Node<'_>) -> bool {
    if symbol_node(node) {
        return crate::cop::ported::static_symbol(source, node).is_some()
            && crate::cop::ported::ruby_string_parts(source, node)
                .is_none_or(|parts| parts.len() == 1);
    }
    if matches!(node.kind(), "string" | "character" | "heredoc_beginning") {
        return crate::cop::ported::ruby_string_parts(source, node)
            .is_some_and(|parts| parts.len() == 1);
    }
    if matches!(
        node.kind(),
        "true"
            | "false"
            | "nil"
            | "integer"
            | "float"
            | "complex"
            | "rational"
            | "regex_options"
            | "string_content"
            | "escape_sequence"
    ) {
        return true;
    }
    if node.kind() == "unary" {
        return stable_immutable_value(source, node);
    }
    matches!(node.kind(), "array" | "hash" | "pair" | "range" | "regex")
        && code_children(node)
            .into_iter()
            .all(|child| literal(source, child))
}
fn implicit_hash(args: &[Node<'_>]) -> bool {
    !args.is_empty()
        && args
            .iter()
            .all(|arg| matches!(arg.kind(), "pair" | "hash_splat_argument"))
}

pub struct ExpectActual;
impl Cop for ExpectActual {
    fn name(&self) -> &'static str {
        "RSpec/ExpectActual"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn default_exclude(&self) -> &'static [&'static str] {
        &["**/spec/routing/**/*"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"to", b"not_to", b"to_not"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !runner(source, node) {
            return;
        }
        let Some(expect) = call_receiver(node).filter(|expect| {
            bare(source, *expect, b"expect")
                && !safe_send(source, *expect)
                && call_block(*expect).is_none()
        }) else {
            return;
        };
        let actual = call_args(expect);
        if actual.is_empty()
            || !implicit_hash(&actual) && actual.len() != 1
            || actual.iter().any(|actual| !literal(source, *actual))
        {
            return;
        }
        let arguments = call_args(node);
        if arguments.len() != 1 {
            return;
        }
        let matcher = arguments[0];
        if call_block(matcher).is_some() || safe_send(source, matcher) {
            return;
        }
        let expected_args = call_args(matcher);
        let (method, expected, expected_literal) = if matcher.kind() == "binary"
            && matcher
                .child_by_field_name("operator")
                .is_some_and(|operator| node_bytes(source, operator) == b"==")
            && matcher.child_by_field_name("left").is_some_and(|left| {
                bare(source, left, b"be")
                    && call_args(left).is_empty()
                    && call_block(left).is_none()
            }) {
            let expected = matcher.child_by_field_name("right").unwrap();
            (b"be".as_slice(), expected, literal(source, expected))
        } else {
            let Some(method) = name(source, matcher).filter(|_| call_receiver(matcher).is_none())
            else {
                return;
            };
            let Some(expected) = expected_args.first().copied() else {
                return;
            };
            (
                method,
                expected,
                if implicit_hash(&expected_args) {
                    expected_args.iter().all(|arg| literal(source, *arg))
                } else {
                    literal(source, expected)
                },
            )
        };
        if matches!(method, b"route_to" | b"be_routable") {
            return;
        }
        let start = actual[0].start_byte();
        let end = actual.last().unwrap().end_byte();
        let index = report(
            self,
            source,
            start,
            end,
            "Provide the actual value you are testing to `expect(...)`.".into(),
            diagnostics,
        );
        diagnostics[index].correctable =
            matches!(method, b"eq" | b"eql" | b"equal" | b"be") && !expected_literal;
        if !diagnostics[index].correctable
            || actual.len() != 1
            || !stable_immutable_value(source, actual[0])
            || !stable_immutable_value(source, expected)
            || expected_args.len() > 1
        {
            return;
        }
        let old = String::from_utf8_lossy(&source.as_bytes()[start..end]);
        let replacement = String::from_utf8_lossy(node_bytes(source, expected));
        if !preserves_comments(source, expect, start, end, &replacement) {
            return;
        }
        let (target_start, target_end, matcher_replacement) = if method == b"be" {
            (expected.start_byte(), expected.end_byte(), old.into_owned())
        } else {
            (
                matcher.start_byte(),
                matcher.end_byte(),
                format!("{}({old})", String::from_utf8_lossy(method)),
            )
        };
        if !preserves_comments(
            source,
            matcher,
            target_start,
            target_end,
            &matcher_replacement,
        ) {
            return;
        }
        if preserving_replace(
            source,
            expect,
            &mut corrections,
            start,
            end,
            replacement,
            self.name(),
        ) {
            preserving_replace(
                source,
                matcher,
                &mut corrections,
                target_start,
                target_end,
                matcher_replacement,
                self.name(),
            );
            diagnostics[index].corrected = true;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectActual, "cops/rspec/expect_actual");
}
