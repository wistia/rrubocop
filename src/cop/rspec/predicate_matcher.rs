use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::ruby_arguments::arguments;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn inflected(predicate: &str) -> String {
    match predicate {
        "is_a?" => "be_a".into(),
        "instance_of?" => "be_an_instance_of".into(),
        "include?" | "respond_to?" => predicate.trim_end_matches('?').into(),
        "exist?" | "exists?" => "exist".into(),
        _ if predicate.starts_with("has_") => {
            format!("have_{}", &predicate[4..predicate.len() - 1])
        }
        _ => format!("be_{}", predicate.trim_end_matches('?')),
    }
}
fn explicit(matcher: &str) -> String {
    match matcher {
        "be_a" | "be_an" | "be_a_kind_of" | "a_kind_of" | "be_kind_of" => "is_a?".into(),
        "be_an_instance_of" | "be_instance_of" | "an_instance_of" => "instance_of?".into(),
        "include" => "include?".into(),
        "respond_to" => "respond_to?".into(),
        _ if matcher.starts_with("have_") && matcher.len() > 5 => format!("has_{}?", &matcher[5..]),
        _ => format!("{}?", matcher.strip_prefix("be_").unwrap_or("")),
    }
}
fn predicate_matcher(config: &CopConfig, name: &str) -> bool {
    if matches!(
        name,
        "be_truthy"
            | "be_falsey"
            | "be_falsy"
            | "have_attributes"
            | "have_received"
            | "be_between"
            | "be_within"
    ) || config
        .options
        .get("AllowedExplicitMatchers")
        .and_then(|v| v.as_sequence())
        .is_some_and(|values| values.iter().any(|v| v.as_str() == Some(name)))
    {
        return false;
    }
    ((name.starts_with("be_") || name.starts_with("have_")) && !name.ends_with('?'))
        || matches!(name, "include" | "respond_to")
}
fn boolean(source: &SourceFile, matcher: Node<'_>, strict: bool) -> Option<Option<bool>> {
    if call_receiver(matcher).is_some()
        || call_block(matcher).is_some()
        || safe_send(source, matcher)
    {
        return None;
    }
    let name = name(source, matcher)?;
    let args = call_args(matcher);
    match name {
        b"be_truthy" | b"a_truthy_value" if args.is_empty() => Some(Some(true)),
        b"be_falsey" | b"be_falsy" | b"a_falsey_value" | b"a_falsy_value" if args.is_empty() => {
            Some(Some(false))
        }
        b"be" | b"eq" | b"eql" | b"equal"
            if !strict && args.len() == 1 && matches!(args[0].kind(), "true" | "false") =>
        {
            Some(matches!(name, b"be" | b"eq").then(|| args[0].kind() == "true"))
        }
        _ => None,
    }
}
fn known_boolean(source: &SourceFile, receiver: Node<'_>, predicate: &str) -> bool {
    pure_literal(source, receiver)
        && (predicate != "include?" || matches!(receiver.kind(), "array" | "string" | "character"))
        && matches!(
            predicate,
            "empty?"
                | "nil?"
                | "zero?"
                | "positive?"
                | "negative?"
                | "even?"
                | "odd?"
                | "include?"
                | "respond_to?"
                | "is_a?"
                | "instance_of?"
        )
        && !node_bytes(source, receiver).starts_with(b"-")
        && !node_bytes(source, receiver).starts_with(b"+")
}
fn rewrite(
    source: &SourceFile,
    node: Node<'_>,
    edits: &mut [(usize, usize, String)],
) -> Option<String> {
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.0));
    let mut text = node_bytes(source, node).to_vec();
    let mut boundary = node.end_byte();
    for (start, end, replacement) in edits {
        if *start < node.start_byte() || *end > boundary || *end < *start {
            return None;
        }
        text.splice(
            *start - node.start_byte()..*end - node.start_byte(),
            replacement.bytes(),
        );
        boundary = *start;
    }
    let text = String::from_utf8(text).ok()?;
    let parsed = SourceFile::from_bytes("predicate.rb", text.as_bytes().to_vec());
    crate::parse::parse_ruby(&parsed)
        .ok()
        .filter(|tree| !tree.root_node().has_error())?;
    Some(text)
}

pub struct PredicateMatcher;
impl Cop for PredicateMatcher {
    fn name(&self) -> &'static str {
        "RSpec/PredicateMatcher"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !runner(source, node) {
            return;
        }
        let Some(expect) =
            call_receiver(node).filter(|n| bare(source, *n, b"expect") && call_block(*n).is_none())
        else {
            return;
        };
        let actuals = arguments(expect);
        if actuals.len() != 1 {
            return;
        }
        let Some(actual) = actuals[0].node() else {
            return;
        };
        let args = arguments(node);
        let Some(matcher) = args.first().and_then(|arg| arg.node()) else {
            return;
        };
        let strict = config.get_bool("Strict", true);
        if config.get_str("EnforcedStyle", "inflected") == "inflected" {
            let Some(receiver) = call_receiver(actual) else {
                return;
            };
            if safe_send(source, actual) || numbered_block(source, actual) {
                return;
            }
            let Some(predicate) = name(source, actual).filter(|name| name.ends_with(b"?")) else {
                return;
            };
            let Some(truth) = boolean(source, matcher, strict) else {
                return;
            };
            let predicate = String::from_utf8_lossy(predicate);
            let matcher_name = inflected(&predicate);
            let index = report(
                self,
                source,
                node.start_byte(),
                header_or_block_end(node),
                format!("Prefer using `{matcher_name}` matcher over `{predicate}`."),
                diagnostics,
            );
            if corrections.is_none()
                || truth.is_none()
                || call_block(actual).is_some()
                || call_block(node).is_some()
                || !known_boolean(source, receiver, &predicate)
                || predicate == "include?"
                    && call_args(actual).iter().any(|arg| {
                        matches!(
                            arg.kind(),
                            "hash" | "pair" | "array" | "hash_splat_argument"
                        )
                    })
                || !call_args(actual)
                    .iter()
                    .chain(call_args(node).iter().skip(1))
                    .all(|arg| pure_literal(source, *arg))
            {
                return;
            }
            let tail = String::from_utf8_lossy(
                &source.as_bytes()[selector(actual).end_byte()..actual.end_byte()],
            );
            let positive = if name(source, node) == Some(b"to") {
                truth.unwrap()
            } else {
                !truth.unwrap()
            };
            let mut edits = [
                (receiver.end_byte(), actual.end_byte(), String::new()),
                (
                    selector(node).start_byte(),
                    selector(node).end_byte(),
                    if positive { "to" } else { "not_to" }.into(),
                ),
                (
                    matcher.start_byte(),
                    matcher.end_byte(),
                    format!("{matcher_name}{tail}"),
                ),
            ];
            if let Some(replacement) = rewrite(source, node, &mut edits) {
                if preserving_replace(
                    source,
                    node,
                    &mut corrections,
                    node.start_byte(),
                    node.end_byte(),
                    replacement,
                    self.name(),
                ) {
                    diagnostics[index].corrected = true;
                }
            }
        } else {
            if call_receiver(matcher).is_some()
                || safe_send(source, matcher)
                || numbered_block(source, matcher)
            {
                return;
            }
            let Some(matcher_name) = name(source, matcher) else {
                return;
            };
            let matcher_name = String::from_utf8_lossy(matcher_name);
            if !predicate_matcher(config, &matcher_name) {
                return;
            }
            let outer_block = call_block(node).is_some()
                && !numbered_block(source, node)
                && args.len() == 1
                && call_block(matcher).is_none()
                && call_args(matcher).is_empty();
            if !outer_block && matcher_name == "include" && arguments(matcher).len() != 1 {
                return;
            }
            let predicate = explicit(&matcher_name);
            let index = report(
                self,
                source,
                node.start_byte(),
                if outer_block {
                    node.end_byte()
                } else {
                    header_or_block_end(node)
                },
                format!("Prefer using `{predicate}` over `{matcher_name}` matcher."),
                diagnostics,
            );
            if !outer_block
                && node.start_position().row != matcher.start_position().row
                && call_args(matcher)
                    .iter()
                    .any(|arg| arg.kind() == "heredoc_beginning")
            {
                diagnostics[index].correctable = false;
                return;
            }
            if corrections.is_none()
                || outer_block
                || call_block(matcher).is_some()
                || !known_boolean(source, actual, &predicate)
                || predicate == "include?"
                    && call_args(matcher).iter().any(|arg| {
                        matches!(
                            arg.kind(),
                            "hash" | "pair" | "array" | "hash_splat_argument"
                        )
                    })
                || !call_args(matcher)
                    .iter()
                    .chain(call_args(node).iter().skip(1))
                    .all(|arg| pure_literal(source, *arg))
            {
                return;
            }
            let tail = String::from_utf8_lossy(
                &source.as_bytes()[selector(matcher).end_byte()..header_end(matcher)],
            );
            let positive = name(source, node) == Some(b"to");
            let replacement_matcher = match (strict, positive) {
                (true, true) => "be(true)",
                (true, false) => "be(false)",
                (false, true) => "be_truthy",
                (false, false) => "be_falsey",
            };
            let mut edits = [
                (
                    actual.end_byte(),
                    actual.end_byte(),
                    format!(".{predicate}{tail}"),
                ),
                (
                    matcher.start_byte(),
                    matcher.end_byte(),
                    replacement_matcher.into(),
                ),
                (
                    selector(node).start_byte(),
                    selector(node).end_byte(),
                    "to".into(),
                ),
            ];
            if let Some(replacement) = rewrite(source, node, &mut edits) {
                if preserving_replace(
                    source,
                    node,
                    &mut corrections,
                    node.start_byte(),
                    node.end_byte(),
                    replacement,
                    self.name(),
                ) {
                    diagnostics[index].corrected = true;
                }
            }
        }
    }
}
fn header_or_block_end(node: Node<'_>) -> usize {
    if call_block(node).is_some() {
        header_end(node)
    } else {
        node.end_byte()
    }
}
