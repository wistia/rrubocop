use crate::cop::CopConfig;
use crate::cop::shared::{call_method_name, call_receiver, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn argument_nodes(node: Node<'_>) -> Vec<Node<'_>> {
    let list = node.child_by_field_name("arguments").or_else(|| {
        children(node)
            .into_iter()
            .find(|child| child.kind() == "argument_list")
    });
    list.map(children).unwrap_or_default()
}
fn pinned_local(source: &SourceFile, node: Node<'_>) -> bool {
    if !node
        .parent()
        .is_some_and(|parent| parent.kind() == "expression_reference_pattern")
    {
        return false;
    }
    struct Scopes(std::sync::Arc<Vec<crate::model::ScopeData>>);
    let scopes = source.cached(0, || {
        Scopes(
            crate::parse::parse_ruby(source)
                .map(|tree| crate::model::build(source.as_bytes(), tree).scopes)
                .unwrap_or_default(),
        )
    });
    let mut current = node.parent();
    let mut index = 0;
    while let Some(ancestor) = current {
        let kind = match ancestor.kind() {
            "method" | "singleton_method" => Some(crate::model::ScopeKind::Method),
            "class" | "module" | "singleton_class" => Some(crate::model::ScopeKind::ClassLike),
            "block" | "do_block" | "lambda" => Some(crate::model::ScopeKind::Block),
            _ => None,
        };
        if let Some(kind) = kind {
            if let Some(found) = scopes
                .0
                .iter()
                .position(|scope| scope.entered_at == ancestor.start_byte() && scope.kind == kind)
            {
                index = found;
                break;
            }
        }
        current = ancestor.parent();
    }
    std::str::from_utf8(node_bytes(source, node))
        .ok()
        .is_some_and(|name| {
            crate::model::lookup_scope(&scopes.0, index, node.start_byte(), name).is_some()
        })
}
pub(super) fn children(node: Node<'_>) -> Vec<Node<'_>> {
    if node.kind() == "expression_reference_pattern" {
        return node.child_by_field_name("value").into_iter().collect();
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| {
            !matches!(
                child.kind(),
                "comment" | "empty_statement" | "heredoc_body" | "heredoc_end"
            )
        })
        .collect()
}
fn parent(node: Node<'_>) -> Option<Node<'_>> {
    let p = node.parent()?;
    if matches!(p.kind(), "argument_list" | "pattern" | "exceptions") {
        return parent(p);
    }
    if matches!(
        p.kind(),
        "program" | "body_statement" | "block_body" | "then" | "else"
    ) && children(p).len() == 1
    {
        return parent(p);
    }
    Some(p)
}
fn operator<'a>(source: &'a SourceFile, node: Node<'_>) -> &'a [u8] {
    node.child_by_field_name("operator")
        .map_or(b"", |op| node_bytes(source, op))
}
fn variable(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "instance_variable" | "class_variable" | "global_variable"
    ) || (node.kind() == "identifier"
        && (crate::cop::ported::ruby_local_identifier(source, node) || pinned_local(source, node)))
}
fn numeric(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(node.kind(), "integer" | "float" | "complex" | "rational")
        || (node.kind() == "unary"
            && matches!(operator(source, node), b"-" | b"+")
            && node
                .child_by_field_name("operand")
                .is_some_and(|n| matches!(n.kind(), "integer" | "float" | "complex" | "rational")))
}
fn literal(source: &SourceFile, node: Node<'_>) -> bool {
    numeric(source, node)
        || matches!(
            node.kind(),
            "string"
                | "chained_string"
                | "simple_symbol"
                | "delimited_symbol"
                | "regex"
                | "array"
                | "hash"
                | "string_array"
                | "symbol_array"
                | "nil"
                | "true"
                | "false"
                | "heredoc_beginning"
        )
}
fn ordinary_call(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "call" | "identifier" | "element_reference" | "binary" | "unary"
    )
}
fn call_parenthesized(source: &SourceFile, node: Node<'_>) -> bool {
    node.child_by_field_name("arguments").is_some_and(|args| {
        node_bytes(source, args).starts_with(b"(")
            && node
                .child_by_field_name("method")
                .is_none_or(|method| method.end_byte() == args.start_byte())
    })
}
fn keyword(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "return"
            | "break"
            | "next"
            | "yield"
            | "super"
            | "redo"
            | "retry"
            | "self"
            | "if"
            | "unless"
            | "case"
            | "when"
            | "while"
            | "until"
            | "for"
    )
}
fn chained(node: Node<'_>) -> bool {
    parent(node).is_some_and(|p| {
        call_receiver(p) == Some(node)
            || p.child_by_field_name("object") == Some(node)
            || (p.kind() == "binary" && p.child_by_field_name("left") == Some(node))
            || (p.kind() == "unary" && p.child_by_field_name("operand") == Some(node))
    })
}
fn first_argument(node: Node<'_>) -> bool {
    let mut current = Some(node);
    while let Some(n) = current {
        if let Some(p) = parent(n) {
            if matches!(p.kind(), "call" | "yield" | "super")
                && argument_nodes(p).first().is_some_and(|arg| *arg == n)
            {
                return true;
            }
        }
        current = parent(n);
    }
    false
}
fn hash_start(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "hash" {
        return Some(node);
    }
    if node.kind() == "call" {
        return call_receiver(node).and_then(hash_start);
    }
    None
}
fn continued_range_endpoint(source: &SourceFile, node: Node<'_>) -> bool {
    let mut previous = node.prev_named_sibling();
    while previous.is_some_and(|node| node.kind() == "comment") {
        previous = previous.and_then(|node| node.prev_named_sibling());
    }
    let Some(mut range) = previous else {
        return false;
    };
    while range.kind() == "assignment" {
        let Some(right) = range.child_by_field_name("right") else {
            return false;
        };
        range = right;
    }
    if range.kind() != "range"
        || range.child_by_field_name("end").is_some()
        || range.child_by_field_name("begin").is_none()
        || range.end_position().row >= node.start_position().row
    {
        return false;
    }
    source.as_bytes()[range.end_byte()..node.start_byte()]
        .split(|byte| *byte == b'\n')
        .all(|line| {
            line.iter()
                .take_while(|byte| **byte != b'#')
                .all(u8::is_ascii_whitespace)
        })
}

fn allowed(source: &SourceFile, node: Node<'_>, kids: &[Node<'_>], config: &CopConfig) -> bool {
    let Some(first) = kids.first().copied() else {
        return true;
    };
    let p = parent(node);
    if continued_range_endpoint(source, node) {
        return true;
    }
    if let Some(list) = node.parent().filter(|list| list.kind() == "argument_list") {
        if list.byte_range() == node.byte_range()
            && list.parent().is_some_and(|call| {
                call.child_by_field_name("method")
                    .is_some_and(|method| method.end_byte() == node.start_byte())
            })
        {
            return true;
        }
    }
    if p.is_some_and(|p| {
        p.kind() == "assignment"
            && p.child_by_field_name("right") == Some(node)
            && p.child_by_field_name("left")
                .is_some_and(|left| left.kind() == "call")
    }) {
        return true;
    }
    if p.is_some_and(|p| {
        p.kind() == "binary"
            && p.child_by_field_name("right") == Some(node)
            && !matches!(
                operator(source, p),
                b"+" | b"-" | b"*" | b"/" | b"%" | b"**" | b"&&" | b"||" | b"and" | b"or"
            )
    }) {
        return true;
    }
    if p.is_some_and(|p| {
        p.kind() == "unary"
            && operator(source, p) == b"defined?"
            && p.child_by_field_name("operand") == Some(node)
    }) {
        return true;
    }

    if node.kind() == "expression_reference_pattern" {
        return !variable(source, first);
    }
    if p.is_some_and(|p| p.kind() == "rescue" || parent(p).is_some_and(|p| p.kind() == "rescue")) {
        return true;
    }
    if p.is_some_and(|p| p.kind() == "range") {
        return true;
    }
    let bytes = source.as_bytes();
    if p.is_some_and(keyword)
        && (node
            .start_byte()
            .checked_sub(1)
            .and_then(|i| bytes.get(i))
            .is_some_and(u8::is_ascii_lowercase)
            || bytes
                .get(node.end_byte())
                .is_some_and(u8::is_ascii_lowercase))
    {
        return true;
    }
    if kids.len() != 1
        && p.is_some_and(|p| {
            !matches!(
                p.kind(),
                "program"
                    | "body_statement"
                    | "block_body"
                    | "parenthesized_statements"
                    | "method"
                    | "block"
                    | "do_block"
            )
        })
    {
        return true;
    }
    if p.is_some_and(|p| p.kind() == "conditional")
        && config.get_bool("TernaryParenthesesEnabled", true)
        && matches!(
            config.get_str("TernaryParenthesesEnforcedStyle", ""),
            "require_parentheses" | "require_parentheses_when_complex"
        )
    {
        return true;
    }
    if p.is_some_and(|p| {
        matches!(p.kind(), "while_modifier" | "until_modifier")
            && p.child_by_field_name("body")
                .is_some_and(|body| body.kind() == "begin")
    }) {
        return true;
    }
    if p.is_some_and(|p| {
        p.kind() == "binary"
            && operator(source, p) == b"=~"
            && p.child_by_field_name("left")
                .is_some_and(|left| left.kind() == "regex")
    }) {
        return true;
    }
    if let Some(p) = p {
        if matches!(p.kind(), "call" | "super" | "yield") && !call_parenthesized(source, p) {
            let args = argument_nodes(p);
            if args.len() == 1 && args[0] == node {
                return true;
            }
            if args.first() == Some(&node) && p.child_by_field_name("block").is_some() {
                return true;
            }
        }
        if matches!(p.kind(), "return" | "next" | "break")
            && p.start_position().row != p.end_position().row
        {
            return true;
        }
    }
    if hash_start(first).is_some() && first_argument(node) {
        let mut current = parent(node);
        let mut root = None;
        while let Some(n) = current {
            if n.kind() == "call" && operator(source, n) != b"&." {
                root = Some(n);
            }
            current = parent(n);
        }
        if root.is_none_or(|root| !call_parenthesized(source, root)) {
            return true;
        }
    }
    false
}
fn redundant_call(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() == "identifier" {
        return !variable(source, node);
    }
    if node.child_by_field_name("block").is_some() {
        return false;
    }
    if node.kind() == "unary" && operator(source, node) == b"not" {
        return false;
    }
    if node.kind() == "element_reference" {
        return node.child_by_field_name("object").is_some_and(|recv| {
            matches!(
                recv.kind(),
                "array" | "hash" | "string_array" | "symbol_array"
            ) || (recv.kind() == "string"
                && !children(recv).iter().any(|n| n.kind() == "interpolation"))
                || ((recv.kind() == "call"
                    || (recv.kind() == "identifier" && !variable(source, recv)))
                    && argument_nodes(recv).is_empty())
        });
    }
    node.kind() == "call" && (argument_nodes(node).is_empty() || call_parenthesized(source, node))
}
fn call_starts_int(node: Node<'_>) -> bool {
    let mut current = node;
    while let Some(receiver) = call_receiver(current) {
        current = receiver;
    }
    current.kind() == "integer"
}
pub(super) fn message(
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
) -> Option<&'static str> {
    let kids = children(node);
    if allowed(source, node, &kids, config) {
        return None;
    }
    let first = *kids.first()?;
    let p = parent(node);
    let text = node_bytes(source, first);
    if matches!(text, b"__FILE__" | b"__LINE__" | b"__ENCODING__")
        || matches!(first.kind(), "redo" | "retry" | "self")
    {
        return Some("a keyword");
    }
    if matches!(
        first.kind(),
        "return" | "next" | "break" | "yield" | "super"
    ) || (first.kind() == "call" && call_method_name(source, first) == Some(b"super"))
    {
        let args = argument_nodes(first);
        let parenthesized_args = children(first)
            .first()
            .is_some_and(|args| node_bytes(source, *args).starts_with(b"("));
        if args.is_empty() || parenthesized_args || call_parenthesized(source, first) {
            return Some("a keyword");
        }
    }
    if first.kind() == "unary"
        && operator(source, first) == b"defined?"
        && first
            .child_by_field_name("operand")
            .is_some_and(|arg| arg.kind() == "parenthesized_statements")
    {
        return Some("a keyword");
    }
    if literal(source, first) {
        let negative = numeric(source, first) && text.starts_with(b"-");
        if !(negative
            && p.is_some_and(|p| {
                p.kind() == "binary"
                    && operator(source, p) == b"**"
                    && p.child_by_field_name("left") == Some(node)
            }))
        {
            return Some("a literal");
        }
    }
    if variable(source, first) {
        return Some("a variable");
    }
    if matches!(first.kind(), "constant" | "scope_resolution") {
        return Some("a constant");
    }
    if first.kind() == "lambda"
        || (first.kind() == "call"
            && matches!(call_method_name(source, first), Some(b"lambda" | b"proc"))
            && first
                .child_by_field_name("block")
                .is_some_and(|block| block.kind() == "block"))
    {
        return Some("an expression");
    }
    if p.is_some_and(|p| p.kind() == "interpolation") {
        return Some("an interpolated expression");
    }
    if first.kind() == "binary" {
        let op = operator(source, first);
        if matches!(op, b"&&" | b"||" | b"and" | b"or") {
            if chained(node)
                || (matches!(op, b"and" | b"or") && p.is_some())
                || (first.start_position().row != first.end_position().row
                    && config.get_bool("ParenthesesAroundConditionEnabled", true)
                    && config.get_bool("AllowInMultilineConditions", false))
                || p.is_some_and(|p| {
                    (p.kind() == "assignment"
                        && p.child_by_field_name("left")
                            .is_some_and(|left| left.kind() == "element_reference"))
                        || matches!(
                            p.kind(),
                            "binary"
                                | "call"
                                | "element_reference"
                                | "splat_argument"
                                | "hash_splat_argument"
                                | "conditional"
                        )
                })
            {
                return None;
            }
            return Some("a logical expression");
        }
        if matches!(op, b"==" | b"===" | b"!=" | b">" | b">=" | b"<" | b"<=") {
            return p.is_none().then_some("a comparison expression");
        }
    }
    if first.kind() == "unary" && !numeric(source, first) && operator(source, first) != b"defined?"
    {
        if chained(node) {
            return None;
        }
        let mut inner = first;
        while inner.kind() == "unary" && operator(source, inner) != b"not" {
            inner = inner.child_by_field_name("operand")?;
        }
        if (inner.kind() == "identifier" && !variable(source, inner))
            || inner.kind() == "call"
            || (inner.kind() == "unary" && operator(source, inner) == b"not")
        {
            if !redundant_call(source, inner) {
                return None;
            }
        }
        return Some("a unary operation");
    }
    if ordinary_call(first) && redundant_call(source, first) {
        if call_starts_int(first)
            && p.is_some_and(|p| p.kind() == "unary" && matches!(operator(source, p), b"-" | b"+"))
        {
            return None;
        }
        return Some("a method call");
    }
    None
}
