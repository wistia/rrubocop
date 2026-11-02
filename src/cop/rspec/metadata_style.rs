use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::ruby_arguments::{Argument, arguments};
use crate::cop::shared::push_replace;
use crate::cop::{Cop, CopConfig};
use crate::correction::{Correction, CorrectionSet};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn symbol(source: &SourceFile, node: Node<'_>) -> Option<String> {
    crate::cop::ported::static_symbol(source, node).filter(|_| {
        crate::cop::ported::ruby_string_parts(source, node).is_none_or(|parts| parts.len() == 1)
    })
}
fn string(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(node.kind(), "string" | "character" | "heredoc_beginning")
        && !symbol_node(node)
        && crate::cop::ported::ruby_string_parts(source, node).is_some_and(|parts| parts.len() == 1)
}
fn left_space(bytes: &[u8], mut offset: usize) -> usize {
    while offset > 0 && matches!(bytes[offset - 1], b' ' | b'\t') {
        offset -= 1;
    }
    while offset > 0 && bytes[offset - 1] == b'\n' {
        offset -= 1;
    }
    offset
}
fn right_space(bytes: &[u8], mut offset: usize) -> usize {
    while matches!(bytes.get(offset), Some(b' ' | b'\t')) {
        offset += 1;
    }
    while bytes.get(offset) == Some(&b'\n') {
        offset += 1;
    }
    offset
}
fn removal(source: &SourceFile, pair: Node<'_>, before: bool, after: bool) -> (usize, usize) {
    let bytes = source.as_bytes();
    if before {
        let mut start = left_space(bytes, pair.start_byte());
        while start > 0 && bytes[start - 1] == b',' {
            start -= 1;
        }
        (start, pair.end_byte())
    } else if after {
        let mut end = pair.end_byte();
        while bytes.get(end) == Some(&b',') {
            end += 1;
        }
        (pair.start_byte(), right_space(bytes, end))
    } else {
        (pair.start_byte(), pair.end_byte())
    }
}
fn key_conflict(source: &SourceFile, key: Node<'_>, value: &str) -> bool {
    if let Some(key) = crate::cop::ported::static_symbol(source, key) {
        return key == value;
    }
    if symbol_node(key) {
        return true;
    }
    if key.kind() == "parenthesized_statements" {
        let children = code_children(key);
        return children
            .last()
            .is_some_and(|key| key_conflict(source, *key, value));
    }
    !matches!(
        key.kind(),
        "string"
            | "chained_string"
            | "character"
            | "heredoc_beginning"
            | "subshell"
            | "integer"
            | "float"
            | "rational"
            | "complex"
            | "nil"
            | "true"
            | "false"
            | "array"
            | "hash"
            | "range"
            | "regex"
    )
}
fn pair_text(value: &str) -> Option<String> {
    let text = format!("{value}: true");
    let source = SourceFile::from_bytes("metadata.rb", format!("{{{text}}}").into_bytes());
    let tree = crate::parse::parse_ruby(&source).ok()?;
    if tree.root_node().has_error() {
        return None;
    }
    let hash = tree.root_node().named_child(0)?;
    let pairs = code_children(hash);
    if pairs.len() != 1 || pairs[0].kind() != "pair" {
        return None;
    }
    let key = pairs[0].child_by_field_name("key")?;
    (symbol(&source, key).as_deref() == Some(value)).then_some(text)
}
fn correct(
    source: &SourceFile,
    node: Node<'_>,
    owner: usize,
    remove: (usize, usize),
    insert: (usize, String),
    corrections: &mut Option<&mut Vec<Correction>>,
    cop: &'static str,
) -> bool {
    if corrections.is_none() || !preserves_comments(source, node, remove.0, remove.1, "") {
        return false;
    }
    let mut edits = Vec::new();
    if insert.0 == remove.0 || insert.0 == remove.1 {
        push_replace(&mut Some(&mut edits), remove.0, remove.1, insert.1, cop);
    } else {
        push_replace(&mut Some(&mut edits), remove.0, remove.1, "", cop);
        push_replace(&mut Some(&mut edits), insert.0, insert.0, insert.1, cop);
    }
    for edit in &mut edits {
        edit.owner = Some(owner);
    }
    let candidate = SourceFile::from_bytes(
        "metadata.rb",
        CorrectionSet::from_vec(edits.clone()).apply(source.as_bytes()),
    );
    if crate::parse::parse_ruby(&candidate)
        .ok()
        .is_none_or(|tree| tree.root_node().has_error())
    {
        return false;
    }
    corrections.as_deref_mut().unwrap().extend(edits);
    true
}
pub struct MetadataStyle;
impl Cop for MetadataStyle {
    fn name(&self) -> &'static str {
        "RSpec/MetadataStyle"
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
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) || !metadata_call(source, node) {
            return;
        }
        let args = arguments(node);
        if args.len() < 2 {
            return;
        }
        let last = args.last().unwrap();
        let hash = last.hash.then_some(last);
        let style = config.get_str("EnforcedStyle", "symbol");
        let mut corrected = false;
        if style == "symbol" {
            let Some(hash) = hash else {
                return;
            };
            let parts = hash.parts();
            for (position, pair) in parts.iter().copied().enumerate() {
                if pair.kind() != "pair" {
                    continue;
                }
                let Some(key) = pair.child_by_field_name("key") else {
                    continue;
                };
                if pair
                    .child_by_field_name("value")
                    .is_none_or(|value| value.kind() != "true")
                {
                    continue;
                }
                let Some(value) = symbol(source, key) else {
                    continue;
                };
                let index = report(
                    self,
                    source,
                    pair.start_byte(),
                    pair.end_byte(),
                    "Use symbol style for metadata.".into(),
                    diagnostics,
                );
                let protected = args[1..args.len() - 1].iter().any(|argument| {
                    argument
                        .node()
                        .and_then(|node| symbol(source, node))
                        .as_deref()
                        == Some(value.as_str())
                });
                if corrected
                    || !protected
                        && parts[position + 1..].iter().any(|prior| {
                            prior.kind() == "hash_splat_argument"
                                || prior.kind() == "pair"
                                    && prior
                                        .child_by_field_name("value")
                                        .is_none_or(|value| value.kind() != "true")
                                    && prior
                                        .child_by_field_name("key")
                                        .is_some_and(|key| key_conflict(source, key, &value))
                        })
                {
                    continue;
                }
                let remove = removal(
                    source,
                    pair,
                    !hash.braces || position > 0,
                    position + 1 < parts.len(),
                );
                let previous = &args[args.len() - 2];
                if correct(
                    source,
                    node,
                    pair.start_byte(),
                    remove,
                    (previous.end(), format!(", {}", ruby_symbol(&value))),
                    &mut corrections,
                    self.name(),
                ) {
                    diagnostics[index].corrected = true;
                    corrected = true;
                }
            }
        } else {
            let include_last = last.node().and_then(|node| symbol(source, node)).is_some();
            let mut symbols = args[1..args.len() - usize::from(!include_last)].to_vec();
            if symbols
                .first()
                .and_then(Argument::node)
                .is_some_and(|node| string(source, node))
            {
                symbols.remove(0);
            }
            for argument in symbols.into_iter().rev() {
                let index = report(
                    self,
                    source,
                    argument.start(),
                    argument.end(),
                    "Use hash style for metadata.".into(),
                    diagnostics,
                );
                let ambiguous = hash.is_none() && !include_last;
                diagnostics[index].correctable = !ambiguous;
                if corrected || ambiguous {
                    continue;
                }
                let Some(value) = argument.node().and_then(|node| symbol(source, node)) else {
                    continue;
                };
                let Some(pair) = pair_text(&value) else {
                    continue;
                };
                let remove = removal(source, argument.nodes[0], true, false);
                let insert = if let Some(hash) = hash {
                    let parts = hash.parts();
                    if let Some(last) = parts.last() {
                        (last.end_byte(), format!(", {pair}"))
                    } else {
                        (hash.start() + 1, format!(" {pair} "))
                    }
                } else {
                    let arguments = node.child_by_field_name("arguments").unwrap();
                    let parentheses = arguments
                        .child(0)
                        .is_some_and(|token| !token.is_named() && token.kind() == "(");
                    (
                        arguments.end_byte() - usize::from(parentheses),
                        format!(", {pair}"),
                    )
                };
                if correct(
                    source,
                    node,
                    argument.start(),
                    remove,
                    insert,
                    &mut corrections,
                    self.name(),
                ) {
                    diagnostics[index].corrected = true;
                    corrected = true;
                }
            }
        }
    }
}
