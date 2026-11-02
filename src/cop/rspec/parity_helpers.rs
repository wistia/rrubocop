use crate::cop::Cop;
use crate::cop::shared::{call_method_name, call_receiver, method_node, node_bytes};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn name<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    if node.kind() == "identifier" {
        if node.parent().is_some_and(|parent| {
            ["method", "name"]
                .iter()
                .any(|field| parent.child_by_field_name(field) == Some(node))
                || parent.kind() == "simple_symbol"
                || parent.kind() == "assignment" && parent.child_by_field_name("left") == Some(node)
        }) {
            return None;
        }
        crate::cop::ported::bare_call_identifier(source, node).then(|| node_bytes(source, node))
    } else if node.kind() == "call" {
        call_method_name(source, node)
    } else {
        None
    }
}

pub(super) fn bare(source: &SourceFile, node: Node<'_>, expected: &[u8]) -> bool {
    call_receiver(node).is_none() && name(source, node) == Some(expected)
}
pub(super) fn runner(source: &SourceFile, node: Node<'_>) -> bool {
    !safe_send(source, node)
        && name(source, node).is_some_and(|n| [b"to".as_slice(), b"not_to", b"to_not"].contains(&n))
}
pub(super) fn symbol<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    match node.kind() {
        "simple_symbol" | "symbol" => node_bytes(source, node).strip_prefix(b":"),
        "delimited_symbol" => {
            let quoted = node_bytes(source, node).strip_prefix(b":")?;
            if quoted.len() < 2 || quoted.windows(2).any(|bytes| bytes == b"#{") {
                return None;
            }
            Some(&quoted[1..quoted.len() - 1])
        }
        "string" if !node_bytes(source, node).windows(2).any(|b| b == b"#{") => {
            let bytes = node_bytes(source, node);
            (bytes.len() >= 2).then(|| &bytes[1..bytes.len() - 1])
        }
        _ => None,
    }
}
pub(super) fn header_end(node: Node<'_>) -> usize {
    node.child_by_field_name("arguments").map_or_else(
        || method_node(node).unwrap_or(node).end_byte(),
        |args| args.end_byte(),
    )
}

pub(super) fn report(
    cop: &dyn Cop,
    source: &SourceFile,
    start: usize,
    end: usize,
    message: String,
    diagnostics: &mut Vec<Diagnostic>,
) -> usize {
    let (line, column) = source.offset_to_line_col(start);
    let mut diagnostic = cop.diagnostic(source, line, column, message);
    diagnostic.highlight_length = if source.offset_to_line_col(end).0 == line {
        source
            .offset_to_line_col(end)
            .1
            .saturating_sub(column)
            .max(1)
    } else {
        source
            .line_start(line + 1)
            .unwrap_or(source.as_bytes().len())
            .saturating_sub(start + 1)
            .max(1)
    };
    diagnostics.push(diagnostic);
    diagnostics.len() - 1
}
pub(super) fn selector(node: Node<'_>) -> Node<'_> {
    method_node(node).unwrap_or(node)
}

pub(super) fn local_binding(source: &SourceFile, node: Node<'_>) -> bool {
    crate::cop::ported::local_identifier(source, node)
}

pub(super) fn hooks() -> &'static [&'static [u8]] {
    &[
        b"before",
        b"after",
        b"around",
        b"prepend_before",
        b"append_before",
        b"prepend_after",
        b"append_after",
    ]
}
pub(super) fn expectations() -> &'static [&'static [u8]] {
    &[
        b"are_expected",
        b"should",
        b"should_not",
        b"should_receive",
        b"should_not_receive",
        b"expect",
        b"expect_any_instance_of",
        b"is_expected",
    ]
}
pub(super) fn metadata_call(source: &SourceFile, node: Node<'_>) -> bool {
    use crate::cop::rspec::helpers::{bare_rspec_call, call_block, is_example, is_group};
    let Some(method) = name(source, node) else {
        return false;
    };
    if !(is_example(method) || is_group(method) || hooks().contains(&method)) {
        return false;
    }
    if bare_rspec_call(source, node).is_some() && call_block(node).is_some() {
        return true;
    }
    if !hooks().contains(&method) {
        return false;
    }
    let Some(receiver) = call_receiver(node) else {
        return false;
    };
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if bare_rspec_call(source, parent) == Some(b"configure") {
            if let Some(parameters) =
                call_block(parent).and_then(|b| b.child_by_field_name("parameters"))
            {
                let mut cursor = parameters.walk();
                return parameters.named_children(&mut cursor).any(|parameter| {
                    node_bytes(source, parameter) == node_bytes(source, receiver)
                });
            }
        }
        ancestor = parent.parent();
    }
    false
}

pub(super) fn call_args(node: Node<'_>) -> Vec<Node<'_>> {
    crate::cop::shared::argument_nodes(node)
        .into_iter()
        .filter(|child| child.kind() != "comment")
        .collect()
}
pub(super) fn code_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .collect()
}
pub(super) fn preserves_comments(
    source: &SourceFile,
    node: Node<'_>,
    start: usize,
    end: usize,
    replacement: &str,
) -> bool {
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        if current.kind() == "comment"
            && current.start_byte() >= start
            && current.end_byte() <= end
            && !replacement
                .as_bytes()
                .windows(current.end_byte() - current.start_byte())
                .any(|part| part == node_bytes(source, current))
        {
            return false;
        }
        let mut cursor = current.walk();
        pending.extend(current.named_children(&mut cursor));
    }
    true
}
pub(super) fn preserving_replace(
    source: &SourceFile,
    node: Node<'_>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    start: usize,
    end: usize,
    replacement: impl Into<String>,
    cop: &'static str,
) -> bool {
    let replacement = replacement.into();
    if !preserves_comments(source, node, start, end, &replacement) {
        return false;
    }
    crate::cop::shared::push_replace(corrections, start, end, replacement, cop)
}

pub(super) fn is_example_group(method: &[u8]) -> bool {
    crate::cop::rspec::helpers::is_group(method)
        && ![
            b"shared_context".as_slice(),
            b"shared_examples",
            b"shared_examples_for",
        ]
        .contains(&method)
}
pub(super) fn top_groups<'a>(source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    crate::cop::rspec::helpers::top_level_groups(source, root)
}
pub(super) fn literal_starts_with(source: &SourceFile, node: Node<'_>, prefixes: &[u8]) -> bool {
    if let Some(value) = crate::cop::ported::static_literal(source, node) {
        return value
            .as_bytes()
            .first()
            .is_some_and(|byte| prefixes.contains(byte));
    }
    if node.kind() == "chained_string" {
        return code_children(node)
            .first()
            .is_some_and(|first| literal_starts_with(source, *first, prefixes));
    }
    code_children(node)
        .first()
        .filter(|child| child.kind() == "string_content")
        .and_then(|child| node_bytes(source, *child).first())
        .is_some_and(|byte| prefixes.contains(byte))
}

pub(super) fn compiled_patterns(config: &crate::cop::CopConfig, key: &str) -> Vec<regex::Regex> {
    config
        .options
        .get(key)
        .and_then(|value| value.as_sequence())
        .into_iter()
        .flatten()
        .filter_map(|pattern| pattern.as_str())
        .filter_map(|pattern| regex::Regex::new(pattern).ok())
        .collect()
}

pub(super) fn metadata_pairs(node: Node<'_>) -> Vec<Node<'_>> {
    call_args(node)
        .into_iter()
        .flat_map(|arg| {
            if arg.kind() == "hash" {
                code_children(arg)
            } else {
                vec![arg]
            }
        })
        .filter(|arg| arg.kind() == "pair")
        .collect()
}
pub(super) fn ignored_metadata(
    source: &SourceFile,
    node: Node<'_>,
    config: &crate::cop::CopConfig,
) -> bool {
    let Some(ignored) = config
        .options
        .get("IgnoredMetadata")
        .and_then(|value| value.as_mapping())
    else {
        return false;
    };
    metadata_pairs(node).iter().any(|pair| {
        let Some(key) = pair.child_by_field_name("key") else {
            return false;
        };
        let Some(value) = pair.child_by_field_name("value") else {
            return false;
        };
        if !matches!(
            key.kind(),
            "hash_key_symbol" | "simple_symbol" | "delimited_symbol" | "symbol"
        ) || !matches!(
            value.kind(),
            "simple_symbol" | "delimited_symbol" | "symbol"
        ) {
            return false;
        }
        let key_bytes = if key.kind() == "hash_key_symbol" {
            Some(node_bytes(source, key))
        } else {
            symbol(source, key)
        };
        let Some(key) = key_bytes else {
            return false;
        };
        let Some(value) = symbol(source, value) else {
            return false;
        };
        ignored
            .get(serde_yml::Value::String(
                String::from_utf8_lossy(key).into_owned(),
            ))
            .and_then(|entry| entry.as_sequence())
            .is_some_and(|allowed| {
                allowed
                    .iter()
                    .filter_map(|value| value.as_str())
                    .any(|entry| entry.as_bytes() == value)
            })
    })
}
pub(super) fn skipped_metadata(source: &SourceFile, node: Node<'_>) -> bool {
    if call_args(node)
        .into_iter()
        .any(|arg| symbol_node(arg) && matches!(symbol(source, arg), Some(b"skip" | b"pending")))
    {
        return true;
    }
    metadata_pairs(node).iter().any(|pair| {
        let Some(key) = pair.child_by_field_name("key") else {
            return false;
        };
        let Some(value) = pair.child_by_field_name("value") else {
            return false;
        };
        let key = if key.kind() == "hash_key_symbol" {
            Some(node_bytes(source, key))
        } else {
            symbol(source, key)
        };
        matches!(key, Some(b"skip" | b"pending"))
            && matches!(value.kind(), "true" | "string" | "chained_string")
    })
}
pub(super) fn same_expression(source: &SourceFile, left: Node<'_>, right: Node<'_>) -> bool {
    if let (Some(a), Some(b)) = (rational_value(source, left), rational_value(source, right)) {
        return a == b;
    }
    if let (Some(a), Some(b)) = (integer_value(source, left), integer_value(source, right)) {
        return a == b;
    }
    let literal_kind = |node: Node<'_>| {
        if symbol_node(node) {
            Some("symbol")
        } else if matches!(
            node.kind(),
            "string" | "chained_string" | "heredoc_beginning"
        ) {
            Some("string")
        } else {
            None
        }
    };
    if let Some(kind) = literal_kind(left) {
        if Some(kind) == literal_kind(right) {
            if let (Some(a), Some(b)) = (
                crate::cop::ported::static_literal(source, left),
                crate::cop::ported::static_literal(source, right),
            ) {
                return a == b;
            }
        }
    }
    let send = |node: Node<'_>| {
        node.kind() == "call"
            || node.kind() == "identifier"
                && name(source, node).is_some()
                && !crate::cop::ported::local_identifier(source, node)
    };
    if send(left) && send(right) {
        let same_receiver = match (call_receiver(left), call_receiver(right)) {
            (None, None) => true,
            (Some(a), Some(b)) => same_expression(source, a, b),
            _ => false,
        };
        let same_block = match (
            crate::cop::rspec::helpers::call_block(left),
            crate::cop::rspec::helpers::call_block(right),
        ) {
            (None, None) => true,
            (Some(a), Some(b)) => same_expression(source, a, b),
            _ => false,
        };
        return same_receiver
            && name(source, left) == name(source, right)
            && safe_send(source, left) == safe_send(source, right)
            && same_arguments(source, &call_args(left), &call_args(right))
            && same_block;
    }
    let body = |node: Node<'_>| matches!(node.kind(), "body_statement" | "block_body");
    if body(left) && code_children(left).len() == 1 {
        return same_expression(source, code_children(left)[0], right);
    }
    if body(right) && code_children(right).len() == 1 {
        return same_expression(source, left, code_children(right)[0]);
    }
    let block = |node: Node<'_>| matches!(node.kind(), "block" | "do_block");
    if left.kind() != right.kind() && !(body(left) && body(right) || block(left) && block(right)) {
        return false;
    }
    if left.kind() == "regex" {
        return same_regex(source, left, right);
    }
    if left.kind() == "float" {
        return String::from_utf8_lossy(node_bytes(source, left))
            .replace('_', "")
            .parse::<f64>()
            .ok()
            == String::from_utf8_lossy(node_bytes(source, right))
                .replace('_', "")
                .parse::<f64>()
                .ok();
    }
    let a = code_children(left);
    let b = code_children(right);
    if a.is_empty() || b.is_empty() {
        return node_bytes(source, left) == node_bytes(source, right);
    }
    if let (Some(a), Some(b)) = (
        left.child_by_field_name("operator"),
        right.child_by_field_name("operator"),
    ) {
        if node_bytes(source, a) != node_bytes(source, b) {
            return false;
        }
    }
    a.len() == b.len()
        && a.iter()
            .zip(&b)
            .all(|(a, b)| same_expression(source, *a, *b))
}

pub(super) fn integer_value(source: &SourceFile, node: Node<'_>) -> Option<num_bigint::BigInt> {
    if node.kind() == "unary" {
        let operator = node.child_by_field_name("operator")?;
        let operand = node.child_by_field_name("operand")?;
        let value = integer_value(source, operand)?;
        return match node_bytes(source, operator) {
            b"-" => Some(-value),
            b"+" => Some(value),
            _ => None,
        };
    }
    if node.kind() != "integer" {
        return None;
    }
    let text = String::from_utf8_lossy(node_bytes(source, node)).replace('_', "");
    let (radix, digits) =
        if let Some(value) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            (16, value)
        } else if let Some(value) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
            (2, value)
        } else if let Some(value) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
            (8, value)
        } else if let Some(value) = text.strip_prefix("0d").or_else(|| text.strip_prefix("0D")) {
            (10, value)
        } else if text.starts_with('0') && text.len() > 1 {
            (8, &text[1..])
        } else {
            (10, text.as_str())
        };
    num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix)
}

pub(super) fn symbol_node(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "simple_symbol" | "delimited_symbol" | "symbol" | "hash_key_symbol"
    ) || node.kind() == "string"
        && node.parent().is_some_and(|parent| {
            parent.kind() == "pair" && parent.child_by_field_name("key") == Some(node)
        })
        && node
            .next_sibling()
            .is_some_and(|operator| operator.kind() == ":")
}

pub(super) fn constant_description(value: &str) -> bool {
    static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    PATTERN
        .get_or_init(|| regex::Regex::new(r"(?m)^(?:(?:::)?[A-Z][A-Za-z0-9_]*)+$").unwrap())
        .is_match(value)
}

pub(super) fn contains_bare_method(source: &SourceFile, node: Node<'_>, method: &[u8]) -> bool {
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        if bare(source, current, method) {
            return true;
        }
        let mut cursor = current.walk();
        pending.extend(current.named_children(&mut cursor));
    }
    false
}
pub(super) fn inside_root_spec_group(source: &SourceFile, node: Node<'_>) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent.kind() == "program" {
            return crate::cop::rspec::helpers::bare_rspec_call(source, current)
                .is_some_and(crate::cop::rspec::helpers::is_group)
                && crate::cop::rspec::helpers::call_block(current).is_some();
        }
        current = parent;
    }
    false
}
pub(super) fn ruby_string(value: &str) -> String {
    let mut output = String::from("\"");
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\x07' => output.push_str("\\a"),
            '\x08' => output.push_str("\\b"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\x0b' => output.push_str("\\v"),
            '\x0c' => output.push_str("\\f"),
            '\r' => output.push_str("\\r"),
            '\x1b' => output.push_str("\\e"),
            '#' if chars
                .peek()
                .is_some_and(|next| ['{', '@', '$'].contains(next)) =>
            {
                output.push_str("\\#")
            }
            control if control.is_ascii_control() => {
                output.push_str(&format!("\\x{:02X}", control as u32))
            }
            control if control.is_control() => {
                output.push_str(&format!("\\u{:04X}", control as u32))
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}
pub(super) fn ruby_symbol(value: &str) -> String {
    let candidate = format!(":{value}");
    let source = SourceFile::from_bytes("symbol.rb", candidate.as_bytes().to_vec());
    if !matches!(value, "!@" | "~@")
        && crate::parse::parse_ruby(&source).ok().is_some_and(|tree| {
            !tree.root_node().has_error()
                && tree.root_node().named_child_count() == 1
                && tree.root_node().named_child(0).is_some_and(|node| {
                    node.kind() == "simple_symbol" && node.end_byte() == candidate.len()
                })
        })
    {
        candidate
    } else {
        format!(":{}", ruby_string(value))
    }
}
pub(super) fn string_options(config: &crate::cop::CopConfig, key: &str) -> Vec<String> {
    match config.options.get(key) {
        Some(serde_yml::Value::String(value)) => vec![value.clone()],
        Some(serde_yml::Value::Sequence(values)) => values
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}
pub(super) fn variable_style(name: &str, style: &str) -> bool {
    static SNAKE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static CAMEL: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    if style == "camelCase" {
        CAMEL
            .get_or_init(|| {
                regex::Regex::new(r"(?m)^@{0,2}(?:_|_?[\p{Ll}][0-9\p{Ll}\p{Lu}]*)[!?=]?$").unwrap()
            })
            .is_match(name)
    } else {
        SNAKE
            .get_or_init(|| regex::Regex::new(r"(?m)^@{0,2}[0-9\p{Ll}_]+[!?=]?$").unwrap())
            .is_match(name)
    }
}
pub(super) fn ruby_regexp_escape(value: &str) -> String {
    let mut output = String::new();
    for c in value.chars() {
        match c {
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\x0b' => output.push_str("\\v"),
            '\x0c' => output.push_str("\\f"),
            c if "\\[]{}()|^$.*+?-# ".contains(c) => {
                output.push('\\');
                output.push(c);
            }
            c => output.push(c),
        }
    }
    output
}

pub(super) fn safe_send(source: &SourceFile, node: Node<'_>) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| node_bytes(source, operator) == b"&.")
}
pub(super) fn runner_including_safe(source: &SourceFile, node: Node<'_>) -> bool {
    name(source, node).is_some_and(|name| [b"to".as_slice(), b"not_to", b"to_not"].contains(&name))
}

pub(super) fn literal_replacement(replacement: &str, expected: &str, symbol: bool) -> bool {
    let source = SourceFile::from_bytes("replacement.rb", replacement.as_bytes().to_vec());
    crate::parse::parse_ruby(&source).ok().is_some_and(|tree| {
        let root = tree.root_node();
        if root.has_error() || root.named_child_count() != 1 {
            return false;
        }
        root.named_child(0).is_some_and(|node| {
            node.end_byte() == replacement.len()
                && if symbol {
                    symbol_node(node)
                } else {
                    node.kind() == "string"
                }
                && crate::cop::ported::static_literal(&source, node).as_deref() == Some(expected)
        })
    })
}

pub(super) fn pure_literal(source: &SourceFile, node: Node<'_>) -> bool {
    if crate::cop::ported::static_literal(source, node).is_some()
        || matches!(
            node.kind(),
            "integer" | "float" | "true" | "false" | "nil" | "hash_key_symbol"
        )
    {
        return true;
    }
    matches!(node.kind(), "array" | "hash" | "pair" | "unary" | "range")
        && code_children(node)
            .iter()
            .all(|child| pure_literal(source, *child))
}

pub(super) fn same_arguments(source: &SourceFile, left: &[Node<'_>], right: &[Node<'_>]) -> bool {
    fn split<'a>(args: &[Node<'a>]) -> (Vec<Node<'a>>, Option<Vec<Node<'a>>>) {
        if let Some(index) = args.iter().position(|arg| arg.kind() == "pair") {
            return (args[..index].to_vec(), Some(args[index..].to_vec()));
        }
        if let Some(last) = args.last().filter(|last| last.kind() == "hash") {
            return (args[..args.len() - 1].to_vec(), Some(code_children(*last)));
        }
        (args.to_vec(), None)
    }
    let (a, hash_a) = split(left);
    let (b, hash_b) = split(right);
    let same = |a: &[Node<'_>], b: &[Node<'_>]| {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(a, b)| same_expression(source, *a, *b))
    };
    same(&a, &b)
        && match (hash_a, hash_b) {
            (None, None) => true,
            (Some(a), Some(b)) => same(&a, &b),
            _ => false,
        }
}

pub(super) fn scoped_examples<'a>(source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    use crate::cop::rspec::helpers::{bare_rspec_call, call_block, is_example, is_group};
    use crate::cop::rspec::separation::numbered_block;
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
    examples
}

pub(super) fn literal_or_const(source: &SourceFile, node: Node<'_>) -> bool {
    if crate::cop::ported::static_literal(source, node).is_some()
        || matches!(
            node.kind(),
            "constant"
                | "scope_resolution"
                | "integer"
                | "float"
                | "rational"
                | "complex"
                | "true"
                | "false"
                | "nil"
                | "hash_key_symbol"
                | "string_content"
                | "escape_sequence"
                | "regex_options"
        )
    {
        return true;
    }
    if node.kind() == "unary" {
        return node
            .child_by_field_name("operator")
            .is_some_and(|operator| matches!(node_bytes(source, operator), b"+" | b"-"))
            && node.child_by_field_name("operand").is_some_and(|operand| {
                matches!(operand.kind(), "integer" | "float" | "rational" | "complex")
            });
    }
    if matches!(
        node.kind(),
        "string"
            | "chained_string"
            | "delimited_symbol"
            | "subshell"
            | "regex"
            | "range"
            | "array"
            | "hash"
            | "pair"
            | "parenthesized_statements"
            | "interpolation"
    ) {
        return code_children(node)
            .into_iter()
            .all(|child| literal_or_const(source, child));
    }
    false
}

fn rational_value(
    source: &SourceFile,
    node: Node<'_>,
) -> Option<(num_bigint::BigInt, num_bigint::BigInt)> {
    use num_bigint::BigInt;
    if node.kind() == "unary" {
        let (value, denominator) = rational_value(source, node.child_by_field_name("operand")?)?;
        return match node_bytes(source, node.child_by_field_name("operator")?) {
            b"-" => Some((-value, denominator)),
            b"+" => Some((value, denominator)),
            _ => None,
        };
    }
    if node.kind() != "rational" {
        return None;
    }
    let number = code_children(node).first().copied()?;
    if let Some(integer) = integer_value(source, number) {
        return Some((integer, BigInt::from(1u8)));
    }
    let text = String::from_utf8_lossy(node_bytes(source, number)).replace('_', "");
    let (whole, fraction) = text.split_once('.')?;
    let numerator = BigInt::parse_bytes(format!("{whole}{fraction}").as_bytes(), 10)?;
    let denominator = BigInt::from(10u8).pow(fraction.len().try_into().ok()?);
    let mut a = numerator.clone();
    let mut b = denominator.clone();
    let zero = BigInt::from(0u8);
    while b != zero {
        let remainder = a % &b;
        a = b;
        b = remainder;
    }
    if a < zero {
        a = -a;
    }
    Some((numerator / &a, denominator / a))
}

enum RegexPart<'a> {
    Text(Vec<u8>),
    Expression(Node<'a>),
}
fn regex_parts<'a>(source: &SourceFile, node: Node<'a>) -> Option<(Vec<RegexPart<'a>>, Vec<u8>)> {
    let mut parts = Vec::new();
    let mut content = Vec::new();
    let opening = node
        .child(0)
        .and_then(|opener| node_bytes(source, opener).last().copied())?;
    let closing = match opening {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        other => other,
    };
    for child in code_children(node) {
        if child.kind() == "interpolation" {
            if !content.is_empty() {
                parts.push(RegexPart::Text(std::mem::take(&mut content)));
            }
            parts.push(RegexPart::Expression(child));
        } else {
            let bytes = node_bytes(source, child);
            if child.kind() == "escape_sequence"
                && bytes.len() == 2
                && bytes[0] == b'\\'
                && (bytes[1] == opening || bytes[1] == closing)
            {
                content.push(bytes[1]);
            } else {
                content.extend_from_slice(bytes);
            }
        }
    }
    if !content.is_empty() {
        parts.push(RegexPart::Text(content));
    }
    let last = node.child(node.child_count().checked_sub(1)?.try_into().ok()?)?;
    let mut options = node_bytes(source, last).get(1..)?.to_vec();
    options.sort_unstable();
    options.dedup();
    Some((parts, options))
}
fn same_regex(source: &SourceFile, left: Node<'_>, right: Node<'_>) -> bool {
    let (Some((a, flags_a)), Some((b, flags_b))) =
        (regex_parts(source, left), regex_parts(source, right))
    else {
        return node_bytes(source, left) == node_bytes(source, right);
    };
    flags_a == flags_b
        && a.len() == b.len()
        && a.iter().zip(&b).all(|(a, b)| match (a, b) {
            (RegexPart::Text(a), RegexPart::Text(b)) => a == b,
            (RegexPart::Expression(a), RegexPart::Expression(b)) => same_expression(source, *a, *b),
            _ => false,
        })
}
