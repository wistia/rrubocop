//! Add noopener without discarding existing link types or routing options.
use crate::cop::shared::ruby_arguments::{Argument, arguments, from_nodes};
use crate::cop::shared::ruby_expression::{plain_string, plain_symbol};
use crate::cop::shared::{call_method_name, call_receiver, named_kids, node_text, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct LinkToBlank;
fn literal(source: &SourceFile, node: Node<'_>) -> Option<Vec<u8>> {
    plain_symbol(source, node)
        .or_else(|| plain_string(source, node))
        .map(String::into_bytes)
        .or_else(|| {
            (node.kind() == "string" && !node_text(source, node).contains('\n'))
                .then(|| crate::cop::ported::static_literal_bytes(source, node))
                .flatten()
        })
}
fn pair_value<'a>(source: &SourceFile, pair: Node<'a>, name: &[u8]) -> Option<Node<'a>> {
    if literal(source, pair.child_by_field_name("key")?)?.as_slice() != name {
        return None;
    }
    pair.child_by_field_name("value")
}
fn noopener(value: &[u8]) -> bool {
    value
        .split(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c))
        .any(|token| matches!(token, b"noopener" | b"noreferrer"))
}
struct Options<'a> {
    argument: Option<usize>,
    parts: Vec<Node<'a>>,
}
fn call<'a>(
    source: &SourceFile,
    node: Node<'a>,
) -> Option<(&'static str, Vec<Argument<'a>>, Option<Node<'a>>)> {
    let (name, args, receiver) = if node.kind() == "binary" {
        let projected = crate::cop::shared::command_unary_plus_call(source, node)?;
        let mut args = from_nodes(projected.arguments.iter().map(|a| a.node));
        if projected.arguments.first().is_some_and(|a| a.unary_plus) {
            if let Some(first) = args.first_mut() {
                first.hash = false;
                first.braces = false;
            }
        }
        (
            source.as_bytes().get(projected.selector.byte_range())?,
            args,
            projected.receiver,
        )
    } else {
        if !super::unknown_env::regular(node, source) || crate::cop::shared::is_setter_call(node) {
            return None;
        }
        (
            call_method_name(source, node)?,
            arguments(node),
            call_receiver(node),
        )
    };
    let name = match name {
        b"link_to" => "link_to",
        b"link_to_if" => "link_to_if",
        b"link_to_unless" => "link_to_unless",
        _ => return None,
    };
    Some((name, args, receiver))
}
fn line_sensitive(source: &SourceFile, node: Node<'_>) -> bool {
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let mut found = false;
    crate::cop::shared::for_each_descendant(root, |n| {
        found |= n.kind() == "identifier"
            && node_text(source, n) == "__LINE__"
            && crate::cop::shared::ruby_expression::integer_ast(source, n).is_some()
    });
    found
}
impl Cop for LinkToBlank {
    fn name(&self) -> &'static str {
        "Rails/LinkToBlank"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"link_to", b"link_to_if", b"link_to_unless"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((name, args, receiver)) = call(source, node) else {
            return;
        };
        let mut groups = Vec::new();
        if let Some(receiver) = receiver.filter(|n| n.kind() == "hash") {
            groups.push(Options {
                argument: None,
                parts: named_kids(receiver),
            });
        }
        groups.extend(
            args.iter()
                .enumerate()
                .filter(|(_, a)| a.hash)
                .map(|(index, arg)| Options {
                    argument: Some(index),
                    parts: arg.parts(),
                }),
        );
        let block_pass = args
            .last()
            .is_some_and(|a| a.node().is_some_and(|n| n.kind() == "block_argument"));
        let count = args.len() - usize::from(block_pass);
        let block = node.child_by_field_name("block").is_some() || block_pass;
        let expected = if name == "link_to" {
            if block { 2 } else { 3 }
        } else {
            4
        };
        for group in groups {
            let Some(blank) = group.parts.iter().copied().find(|p| {
                pair_value(source, *p, b"target")
                    .and_then(|v| literal(source, v))
                    .as_deref()
                    == Some(b"_blank")
            }) else {
                continue;
            };
            if group.parts.iter().any(|p| {
                pair_value(source, *p, b"rel")
                    .and_then(|v| literal(source, v))
                    .is_some_and(|v| noopener(&v))
            }) {
                continue;
            }
            let (line, col) = source.offset_to_line_col(blank.start_byte());
            let mut d = self.diagnostic(
                source,
                line,
                col,
                "Specify a `:rel` option containing noopener.".into(),
            );
            d.correctable = false;
            d.highlight_length =
                super::blank::highlight_length(source, blank.start_byte(), blank.end_byte());
            let rels: Vec<_> = group
                .parts
                .iter()
                .filter(|p| {
                    p.child_by_field_name("key")
                        .and_then(|n| literal(source, n))
                        .as_deref()
                        == Some(b"rel")
                })
                .map(|p| p.child_by_field_name("value"))
                .collect();
            let targets = group
                .parts
                .iter()
                .filter(|p| {
                    p.child_by_field_name("key")
                        .and_then(|n| literal(source, n))
                        .as_deref()
                        == Some(b"target")
                })
                .count();
            let mut edit = None;
            if count == expected
                && group.argument == Some(count - 1)
                && targets == 1
                && rels.len() <= 1
                && group.parts.iter().all(|n| {
                    n.kind() == "pair"
                        && n.child_by_field_name("key")
                            .and_then(|key| literal(source, key))
                            .is_some()
                })
            {
                if let Some(Some(rel)) = rels.first().copied() {
                    if let Some(value) = plain_symbol(source, rel) {
                        edit = Some((
                            rel.start_byte(),
                            rel.end_byte(),
                            crate::cop::shared::ruby_literal::symbol_inspect(&format!(
                                "{value} noopener"
                            )),
                        ));
                    } else if literal(source, rel).is_some() {
                        let raw = node_text(source, rel);
                        if rel.kind() == "string"
                            && (raw.starts_with(['\'', '"']) || raw.starts_with('%'))
                        {
                            edit =
                                Some((rel.end_byte() - 1, rel.end_byte() - 1, " noopener".into()));
                        } else if rel.kind() == "character" {
                            if let Some(value) = plain_string(source, rel) {
                                edit = Some((
                                    rel.start_byte(),
                                    rel.end_byte(),
                                    crate::cop::shared::ruby_literal::string_literal(&format!(
                                        "{value} noopener"
                                    )),
                                ));
                            }
                        }
                    }
                } else if rels.is_empty() {
                    if let Some(last) = group.parts.iter().rev().find(|n| n.kind() == "pair") {
                        let target = pair_value(source, blank, b"target").unwrap();
                        let value = if plain_symbol(source, target).is_some() {
                            ":noopener"
                        } else if node_text(source, target).starts_with('"') {
                            "\"noopener\""
                        } else {
                            "'noopener'"
                        };
                        edit = Some((last.end_byte(), last.end_byte(), format!(", rel: {value}")));
                    }
                }
            }
            if let Some((start, end, replacement)) = edit {
                let mut safe = !config.autocorrect_disabled();
                crate::cop::shared::for_each_descendant(node, |n| {
                    if n.kind() == "comment" && start < n.end_byte() && n.start_byte() < end {
                        safe = false;
                    }
                });
                let changes_lines = source.as_bytes()[start..end]
                    .iter()
                    .filter(|b| **b == b'\n')
                    .count()
                    != replacement.bytes().filter(|b| *b == b'\n').count();
                safe &= !changes_lines || !line_sensitive(source, node);
                if safe {
                    let mut proposed = source.as_bytes().to_vec();
                    proposed.splice(start..end, replacement.bytes());
                    let proposed = SourceFile::from_bytes("link.rb", proposed);
                    safe = crate::parse::parse_ruby(&proposed)
                        .is_ok_and(|t| !t.root_node().has_error());
                }
                d.correctable = safe;
                if safe {
                    d.corrected =
                        push_replace(&mut corrections, start, end, replacement, self.name());
                }
            }
            diagnostics.push(d);
        }
    }
}
