use crate::cop::rspec::helpers::{
    RSPEC_INCLUDE, bare_rspec_call, call_block, is_example, is_group,
};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::ruby_arguments::arguments;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::ruby_expression::plain_symbol;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

type Metadata<'a> = Vec<(Node<'a>, Option<Node<'a>>)>;
struct Hook<'a> {
    node: Node<'a>,
    scope: Option<&'static str>,
    metadata: Metadata<'a>,
}
fn valid_scope(scope: &str) -> bool {
    matches!(scope, "each" | "example" | "context" | "all" | "suite")
}
fn hook<'a>(source: &SourceFile, node: Node<'a>) -> Option<Hook<'a>> {
    let args = arguments(node);
    let first = args.first();
    let scope_name = match first {
        None => None,
        Some(arg) if arg.hash => None,
        Some(arg) => Some(plain_symbol(source, arg.node()?)?),
    };
    let scope = match scope_name.as_deref() {
        None | Some("each" | "example") => Some("each"),
        Some("all" | "context") => Some("context"),
        Some("suite") => Some("suite"),
        _ => None,
    };
    let mut metadata = Vec::new();
    if !numbered_block(source, node) {
        let skip = usize::from(scope_name.as_deref().is_some_and(valid_scope));
        for arg in args.iter().skip(skip) {
            let entries = if arg.hash {
                arg.parts()
                    .iter()
                    .filter(|pair| pair.kind() == "pair")
                    .map(|pair| {
                        let key = pair.child_by_field_name("key")?;
                        let value = pair.child_by_field_name("value")?;
                        Some((key, (value.kind() != "true").then_some(value)))
                    })
                    .collect::<Option<Vec<_>>>()?
            } else {
                let symbol = arg.node()?;
                plain_symbol(source, symbol)?;
                vec![(symbol, None)]
            };
            for (key, value) in entries {
                if let Some(entry) = metadata
                    .iter_mut()
                    .find(|(old, _)| eql_expression(source, *old, key))
                {
                    *entry = (key, value);
                } else {
                    metadata.push((key, value));
                }
            }
        }
    }
    Some(Hook {
        node,
        scope,
        metadata,
    })
}
fn metadata_equal(source: &SourceFile, left: &Metadata<'_>, right: &Metadata<'_>) -> bool {
    left.len() == right.len()
        && left.iter().all(|(key, value)| {
            right.iter().any(|(other_key, other_value)| {
                eql_expression(source, *key, *other_key)
                    && match (value, other_value) {
                        (None, None) => true,
                        (Some(a), Some(b)) => eql_expression(source, *a, *b),
                        _ => false,
                    }
            })
        })
}
fn include(name: &[u8]) -> bool {
    matches!(
        name,
        b"include_examples" | b"it_behaves_like" | b"it_should_behave_like" | b"include_context"
    )
}
fn collect<'a>(source: &SourceFile, node: Node<'a>, out: &mut Vec<Hook<'a>>) {
    for child in code_children(node) {
        let block = call_block(child);
        if block.is_some()
            && call_receiver(child).is_none()
            && name(source, child).is_some_and(|n| hooks().contains(&n))
        {
            if let Some(hook) = hook(source, child) {
                out.push(hook);
            }
            continue;
        }
        if block.is_some() && !numbered_block(source, child) {
            if bare_rspec_call(source, child).is_some_and(is_group)
                || call_receiver(child).is_none()
                    && name(source, child).is_some_and(|n| is_example(n) || include(n))
            {
                continue;
            }
        }
        collect(source, child, out);
    }
}
fn body(node: Node<'_>) -> Vec<Node<'_>> {
    call_block(node)
        .and_then(|b| b.child_by_field_name("body"))
        .map(code_children)
        .unwrap_or_default()
        .into_iter()
        .filter(|n| n.kind() != "heredoc_body")
        .collect()
}
fn safe_body(source: &SourceFile, node: Node<'_>) -> bool {
    if numbered_block(source, node)
        || call_block(node)
            .and_then(|b| b.child_by_field_name("parameters"))
            .is_some_and(|p| !code_children(p).is_empty())
    {
        return false;
    }
    let mut pending = body(node);
    while let Some(child) = pending.pop() {
        if matches!(
            child.kind(),
            "assignment"
                | "operator_assignment"
                | "multiple_assignment"
                | "method"
                | "singleton_method"
                | "class"
                | "module"
                | "return"
                | "break"
                | "next"
                | "redo"
                | "retry"
                | "yield"
                | "super"
                | "for"
                | "rescue"
                | "exception_variable"
                | "case_match"
                | "match_pattern"
                | "test_pattern"
        ) || child.kind() == "identifier" && node_bytes(source, child) == b"__LINE__"
            || bare_rspec_call(source, child).is_some_and(|n| {
                hooks().contains(&n)
                    || is_group(n)
                    || include(n)
                    || matches!(n, b"let" | b"let!" | b"subject" | b"subject!")
            })
        {
            return false;
        }
        pending.extend(code_children(child));
    }
    true
}
fn line_range(source: &SourceFile, node: Node<'_>) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let start = bytes[..node.start_byte()]
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |p| p + 1);
    let end = bytes[node.end_byte()..]
        .iter()
        .position(|b| *b == b'\n')
        .map_or(bytes.len(), |p| node.end_byte() + p + 1);
    (bytes[start..node.start_byte()]
        .iter()
        .all(|b| b.is_ascii_whitespace())
        && bytes[node.end_byte()..end]
            .iter()
            .all(|b| b.is_ascii_whitespace()))
    .then_some((start, end))
}
fn close_start(node: Node<'_>) -> Option<usize> {
    let block = call_block(node)?;
    (0..block.child_count())
        .rev()
        .filter_map(|i| block.child(i as u32))
        .find(|n| matches!(n.kind(), "}" | "end"))
        .map(|n| n.start_byte())
}
fn heredoc_end(source: &SourceFile, node: Node<'_>) -> Option<usize> {
    let mut pending = vec![node];
    let mut end = None;
    while let Some(child) = pending.pop() {
        if child.kind() == "heredoc_beginning" {
            if let Some(offset) = crate::cop::ported::heredoc_end(source, child) {
                end = Some(end.map_or(offset, |previous: usize| previous.max(offset)));
            }
        }
        pending.extend(code_children(child));
    }
    end
}

fn merge(
    source: &SourceFile,
    root: Node<'_>,
    group: &[&Hook<'_>],
) -> Option<(usize, usize, String)> {
    let first = group.first()?;
    let last = group.last()?;
    if !matches!(name(source, first.node), Some(b"before" | b"append_before"))
        || first.scope.is_none()
        || group
            .iter()
            .skip(1)
            .any(|hook| same_expression(source, first.node, hook.node))
        || group.iter().any(|hook| {
            !safe_body(source, hook.node)
                || !call_args(hook.node)
                    .iter()
                    .all(|arg| pure_literal(source, *arg))
        })
    {
        return None;
    }
    let parent = first.node.parent()?;
    if call_block(root)?.child_by_field_name("body") != Some(parent)
        || group.iter().any(|hook| hook.node.parent() != Some(parent))
    {
        return None;
    }
    let siblings = code_children(parent);
    let a = siblings.iter().position(|n| *n == first.node)?;
    let b = siblings.iter().position(|n| *n == last.node)?;
    if b - a + 1 != group.len() {
        return None;
    }
    let bodies = group.iter().map(|hook| body(hook.node)).collect::<Vec<_>>();
    let first_end = bodies[0].last()?.end_byte();
    if heredoc_end(source, first.node).is_some() {
        return None;
    }
    let ranges = group
        .iter()
        .map(|hook| line_range(source, hook.node))
        .collect::<Option<Vec<_>>>()?;
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return None;
    }
    let start = ranges[0].0;
    let end = ranges.last()?.1;
    if crate::cop::shared::collect_comments(parent)
        .iter()
        .any(|n| n.start_byte() >= start && n.start_byte() < end)
    {
        return None;
    }
    let mut insertion = String::new();
    for (hook, body) in group.iter().zip(&bodies).skip(1) {
        let close = close_start(hook.node)?;
        if heredoc_end(source, hook.node).is_some_and(|end| end > close) {
            return None;
        }
        insertion.push('\n');
        if let Some(body) = body.first() {
            insertion
                .push_str(std::str::from_utf8(&source.as_bytes()[body.start_byte()..close]).ok()?);
        }
    }
    let mut text = source.as_bytes()[start..end].to_vec();
    for (a, b) in ranges.iter().skip(1).rev() {
        text.drain(a - start..b - start);
    }
    text.splice(first_end - start..first_end - start, insertion.bytes());
    let text = String::from_utf8(text).ok()?;
    let parsed = SourceFile::from_bytes("hooks.rb", text.as_bytes().to_vec());
    crate::parse::parse_ruby(&parsed)
        .ok()
        .filter(|tree| !tree.root_node().has_error())?;
    Some((start, end, text))
}

pub struct ScatteredSetup;
impl Cop for ScatteredSetup {
    fn name(&self) -> &'static str {
        "RSpec/ScatteredSetup"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_block(node).is_none()
            || numbered_block(source, node)
            || !bare_rspec_call(source, node).is_some_and(|n| {
                is_group(n)
                    && !matches!(
                        n,
                        b"shared_examples" | b"shared_examples_for" | b"shared_context"
                    )
            })
        {
            return;
        }
        let mut hooks = Vec::new();
        collect(source, node, &mut hooks);
        let mut groups: Vec<Vec<&Hook<'_>>> = Vec::new();
        for hook in &hooks {
            if let Some(group) = groups.iter_mut().find(|group| {
                name(source, group[0].node) == name(source, hook.node)
                    && group[0].scope == hook.scope
                    && metadata_equal(source, &group[0].metadata, &hook.metadata)
            }) {
                group.push(hook);
            } else {
                groups.push(vec![hook]);
            }
        }
        for group in groups.into_iter().filter(|group| group.len() > 1) {
            let first_body = !body(group[0].node).is_empty();
            let mut owners = Vec::new();
            for (position, hook) in group.iter().enumerate() {
                let lines = group
                    .iter()
                    .filter(|other| {
                        other.node.start_position().row != hook.node.start_position().row
                    })
                    .map(|other| other.node.start_position().row + 1)
                    .collect::<Vec<_>>();
                let lines = if lines.len() == 1 {
                    format!("line {}", lines[0])
                } else {
                    format!(
                        "lines {}",
                        lines
                            .iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                let method = String::from_utf8_lossy(name(source, hook.node).unwrap());
                let index = report(
                    self,
                    source,
                    hook.node.start_byte(),
                    hook.node.end_byte(),
                    format!(
                        "Do not define multiple `{method}` hooks in the same example group (also defined on {lines})."
                    ),
                    diagnostics,
                );
                if position == 0 || !first_body || same_expression(source, group[0].node, hook.node)
                {
                    diagnostics[index].correctable = false;
                } else {
                    owners.push((hook.node.start_byte(), index));
                }
            }
            if owners.is_empty() || corrections.is_none() {
                continue;
            }
            let Some((start, end, text)) = merge(source, node, &group) else {
                continue;
            };
            for (owner, index) in owners {
                corrections.as_deref_mut().unwrap().push(Correction {
                    start,
                    end,
                    replacement: text.clone(),
                    cop_name: self.name(),
                    cop_index: 0,
                    owner: Some(owner),
                });
                diagnostics[index].corrected = true;
            }
        }
    }
}
