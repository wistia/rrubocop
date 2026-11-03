//! Detect legacy tag calls while preserving unsafe serialization/evaluation contexts.
use crate::cop::shared::ruby_arguments::{Argument, arguments, from_nodes};
use crate::cop::shared::ruby_expression::{plain_string, plain_symbol};
use crate::cop::shared::ruby_inflection::underscore;
use crate::cop::shared::{
    call_method_name, call_receiver, method_node, node_text, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ContentTag;
fn valid_name(value: &str) -> bool {
    value.split('\n').any(|line| {
        let mut bytes = line.bytes();
        bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'-')
            && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}
fn void(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "keygen"
            | "link"
            | "meta"
            | "source"
            | "track"
            | "wbr"
    )
}
fn rest_safe(source: &SourceFile, rest: &[Argument<'_>]) -> bool {
    rest.iter().all(|arg| {
        if !arg.hash || arg.braces {
            return arg.node().is_some_and(|n| n.kind() == "block_argument");
        }
        arg.parts().iter().all(|n| {
            n.kind() == "pair"
                && n.child_by_field_name("key")
                    .and_then(|n| plain_symbol(source, n))
                    .as_deref()
                    != Some("escape")
        })
    })
}
fn root_constant(source: &SourceFile, node: Node<'_>, expected: &str) -> bool {
    node.kind() == "constant" && node_text(source, node) == expected
        || node.kind() == "scope_resolution"
            && node.child_by_field_name("scope").is_none()
            && node
                .child_by_field_name("name")
                .is_some_and(|n| node_text(source, n) == expected)
}
struct MarkedRange {
    owner: usize,
    start: usize,
    end: usize,
}
struct TagFacts {
    xml: bool,
    line: bool,
    ranges: Vec<MarkedRange>,
}
fn file_facts(source: &SourceFile, node: Node<'_>) -> std::sync::Arc<TagFacts> {
    source.cached(0, || {
        let mut root = node;
        while let Some(parent) = root.parent() {
            root = parent;
        }
        let directives = crate::parse::directives::cached(source);
        let mut facts = TagFacts {
            xml: false,
            line: false,
            ranges: Vec::new(),
        };
        crate::cop::shared::for_each_descendant(root, |n| {
            facts.line |= n.kind() == "identifier"
                && node_text(source, n) == "__LINE__"
                && crate::cop::shared::ruby_expression::integer_ast(source, n).is_some();
            facts.xml |= root_constant(source, n, "REXML")
                || n.kind() == "scope_resolution"
                    && n.child_by_field_name("scope")
                        .is_some_and(|s| root_constant(source, s, "Nokogiri"))
                    && n.child_by_field_name("name")
                        .is_some_and(|name| node_text(source, name) == "XML")
                || call_method_name(source, n) == Some(b"XML")
                    && call_receiver(n).is_some_and(|s| root_constant(source, s, "Nokogiri"));
            let Some(candidate) = call(source, n) else {
                return;
            };
            let (line, _) = source.offset_to_line_col(candidate.start);
            if preference(source, &candidate).is_none()
                || directives.suppresses("Rails/ContentTag", line)
            {
                return;
            }
            facts.ranges.push(MarkedRange {
                owner: candidate.start,
                start: candidate.start,
                end: candidate.end,
            });
            for arg in &candidate.args {
                for argument in &arg.nodes {
                    crate::cop::shared::for_each_descendant(*argument, |n| {
                        if n.kind() == "heredoc_beginning"
                            && candidate.start <= n.start_byte()
                            && n.end_byte() <= candidate.end
                        {
                            if let Some((start, end)) = crate::cop::ported::heredoc_span(source, n)
                            {
                                facts.ranges.push(MarkedRange {
                                    owner: candidate.start,
                                    start,
                                    end,
                                });
                            }
                        }
                    });
                }
            }
        });
        facts
    })
}

fn serialized_value_context(node: Node<'_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(n) = ancestor {
        if matches!(
            n.kind(),
            "method" | "singleton_method" | "class" | "module" | "block" | "do_block" | "lambda"
        ) {
            return false;
        }
        if matches!(n.kind(), "pair" | "heredoc_body") {
            return true;
        }
        ancestor = n.parent();
    }
    false
}

struct Call<'a> {
    selector: Node<'a>,
    start: usize,
    end: usize,
    args: Vec<Argument<'a>>,
    first_start: usize,
    first_end: usize,
    unary_plus: bool,
}
fn call<'a>(source: &SourceFile, node: Node<'a>) -> Option<Call<'a>> {
    if node.kind() == "binary" {
        let projected = crate::cop::shared::command_unary_plus_call(source, node)?;
        if projected.receiver.is_some() || node_text(source, projected.selector) != "tag" {
            return None;
        }
        let first = projected.arguments.first()?;
        return Some(Call {
            selector: projected.selector,
            start: projected.start,
            end: projected.end,
            args: from_nodes(projected.arguments.iter().map(|a| a.node)),
            first_start: first.start,
            first_end: first.end,
            unary_plus: first.unary_plus,
        });
    }
    if !super::unknown_env::regular(node, source)
        || call_receiver(node).is_some()
        || call_method_name(source, node) != Some(b"tag")
        || crate::cop::shared::is_setter_call(node)
    {
        return None;
    }
    let args = arguments(node);
    let first = args.first()?;
    Some(Call {
        selector: method_node(node)?,
        start: node.start_byte(),
        end: send_end(node),
        first_start: first.start(),
        first_end: first.end(),
        args,
        unary_plus: false,
    })
}

fn preference<'a>(source: &SourceFile, call: &Call<'a>) -> Option<(Node<'a>, String)> {
    if call.args.is_empty() || call.args.len() >= 3 {
        return None;
    }
    let first = call.args[0].node()?;
    if call.unary_plus
        && crate::cop::shared::ruby_expression::integer_ast(source, first).is_none()
        && crate::cop::shared::ruby_expression::float_value(source, first).is_none()
        && crate::cop::shared::ruby_expression::rational_value(source, first).is_none()
    {
        return None;
    }
    let value = super::literal_value::value(source, first)?;
    if (plain_string(source, first).is_some() || plain_symbol(source, first).is_some())
        && !valid_name(&value)
    {
        return None;
    }
    Some((first, underscore(&value)))
}
fn corrected_ancestor(source: &SourceFile, current: &Call<'_>, node: Node<'_>) -> bool {
    file_facts(source, node).ranges.iter().any(|range| {
        range.owner != current.start && range.start <= current.start && current.end <= range.end
    })
}

impl Cop for ContentTag {
    fn name(&self) -> &'static str {
        "Rails/ContentTag"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn default_exclude(&self) -> &'static [&'static str] {
        &["**/app/models/**/*.rb", "**/config/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"tag"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !config.get_bool("__RailtiesInLockfile", true)
            || config.get_f64(
                "__RailtiesVersion",
                config.get_f64("TargetRailsVersion", 8.1),
            ) < 5.1
        {
            return;
        }
        let Some(call) = call(source, node) else {
            return;
        };
        let args = &call.args;
        let Some((first, preferred)) = preference(source, &call) else {
            return;
        };
        if corrected_ancestor(source, &call, node) {
            return;
        }
        let method = call.selector;
        let end = call.end;
        let rest = &args[1..];
        let replacement = format!(
            "tag.{preferred}({})",
            rest.iter()
                .map(|a| std::str::from_utf8(&source.as_bytes()[a.start()..a.end()]).unwrap_or(""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let (line, col) = source.offset_to_line_col(call.start);
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `tag.{preferred}` instead of `tag({})`.",
                String::from_utf8_lossy(&source.as_bytes()[call.first_start..call.first_end])
            ),
        );
        d.highlight_length = super::blank::highlight_length(source, call.start, end);
        let mut dynamic = false;
        crate::cop::shared::for_each_descendant(first, |n| dynamic |= n.kind() == "interpolation");
        d.correctable = !config.autocorrect_disabled()
            && !dynamic
            && first.kind() != "heredoc_beginning"
            && !serialized_value_context(node)
            && void(&preferred)
            && rest_safe(source, rest);
        crate::cop::shared::for_each_descendant(node, |n| {
            if n.kind() == "comment" && method.start_byte() < n.end_byte() && end > n.start_byte() {
                d.correctable = false;
            }
        });
        if d.correctable {
            let facts = file_facts(source, node);
            let removes_lines = source.as_bytes()[method.start_byte()..end]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
                != replacement.bytes().filter(|b| *b == b'\n').count();
            d.correctable = !facts.xml && !(facts.line && removes_lines);
        }
        if d.correctable {
            let mut proposed = source.as_bytes().to_vec();
            proposed.splice(method.start_byte()..end, replacement.bytes());
            let proposed = SourceFile::from_bytes("tag.rb", proposed);
            d.correctable =
                crate::parse::parse_ruby(&proposed).is_ok_and(|t| !t.root_node().has_error());
        }
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                method.start_byte(),
                end,
                replacement,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
