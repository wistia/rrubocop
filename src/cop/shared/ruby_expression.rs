//! Ruby expression comparison adapted to the pinned legacy AST's literal and send shapes.
use super::{
    argument_nodes, call_method_name, call_receiver, integer_value, named_kids, node_bytes,
    node_text,
};
use crate::parse::source::SourceFile;
use std::borrow::Cow;
use tree_sitter::Node;
fn code_children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect()
}
fn target(node: Node<'_>) -> Node<'_> {
    if node.kind() == "assignment" {
        node.child_by_field_name("left")
            .filter(|n| matches!(n.kind(), "call" | "element_reference"))
            .unwrap_or(node)
    } else {
        node
    }
}
fn operator<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    let bytes = node_bytes(source, node.child_by_field_name("operator")?);
    Some(match bytes {
        b"&&" | b"and" => b"and",
        b"||" | b"or" => b"or",
        other => other,
    })
}
fn name<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<Cow<'a, [u8]>> {
    let node = target(node);
    let bytes: &'a [u8] = match node.kind() {
        "call" => call_method_name(source, node).unwrap_or(b"call"),
        "element_reference" => b"[]",
        "binary" => operator(source, node)?,
        "unary" => match operator(source, node)? {
            b"-" => b"-@",
            b"+" => b"+@",
            b"not" => b"!",
            other => other,
        },
        "identifier" if crate::cop::ported::bare_call_identifier(source, node) => {
            node_bytes(source, node)
        }
        _ => return None,
    };
    if super::is_setter_call(node) {
        let mut bytes = bytes.to_vec();
        bytes.push(b'=');
        Some(Cow::Owned(bytes))
    } else {
        Some(Cow::Borrowed(bytes))
    }
}
fn is_send(source: &SourceFile, node: Node<'_>) -> bool {
    let n = target(node);
    match n.kind() {
        "call" | "element_reference" => true,
        "identifier" => name(source, n).is_some(),
        "binary" => !matches!(operator(source, n), Some(b"and" | b"or")),
        "unary" => {
            operator(source, n) != Some(b"defined?")
                && !(matches!(operator(source, n), Some(b"+" | b"-"))
                    && n.child_by_field_name("operand").is_some_and(|n| {
                        matches!(n.kind(), "integer" | "float" | "rational" | "complex")
                    }))
        }
        _ => false,
    }
}
fn send_receiver(node: Node<'_>) -> Option<Node<'_>> {
    let n = target(node);
    match n.kind() {
        "binary" => n.child_by_field_name("left"),
        "unary" => n.child_by_field_name("operand").map(|operand| {
            if n.child_by_field_name("operator")
                .is_some_and(|op| op.kind() == "not")
                && operand.kind() == "parenthesized_statements"
            {
                let children = code_children(operand);
                if children.len() == 1 {
                    return children[0];
                }
            }
            operand
        }),
        "element_reference" => n.child_by_field_name("object"),
        _ => call_receiver(n),
    }
}
fn call_args(node: Node<'_>) -> Vec<Node<'_>> {
    let n = target(node);
    let mut args = match n.kind() {
        "binary" => n.child_by_field_name("right").into_iter().collect(),
        "element_reference" => code_children(n)
            .into_iter()
            .filter(|child| Some(*child) != n.child_by_field_name("object"))
            .collect(),
        _ => argument_nodes(n),
    };
    if n != node {
        args.extend(node.child_by_field_name("right"));
    }
    args
}
fn safe_send(source: &SourceFile, node: Node<'_>) -> bool {
    target(node)
        .child_by_field_name("operator")
        .is_some_and(|n| node_bytes(source, n) == b"&.")
}
fn call_block(node: Node<'_>) -> Option<Node<'_>> {
    target(node).child_by_field_name("block")
}
fn is_block(node: Node<'_>) -> bool {
    matches!(node.kind(), "block" | "do_block")
}
fn block_part<'a>(node: Node<'a>, field: &str) -> Option<Node<'a>> {
    node.child_by_field_name(field)
        .filter(|n| !code_children(*n).is_empty())
}

fn same_block(source: &SourceFile, left: Node<'_>, right: Node<'_>, strict: bool) -> bool {
    let same_expression = |source: &SourceFile, left: Node<'_>, right: Node<'_>| {
        compare_expression(source, left, right, strict)
    };
    ["parameters", "body"].into_iter().all(|field| {
        match (block_part(left, field), block_part(right, field)) {
            (None, None) => true,
            (Some(a), Some(b)) => same_expression(source, a, b),
            _ => false,
        }
    })
}
#[derive(PartialEq)]
enum StringAst {
    Str(String),
    Dstr(Vec<StringAst>),
}
fn string_shape(source: &SourceFile, node: Node<'_>) -> Option<StringAst> {
    if node.kind() == "chained_string" {
        return code_children(node)
            .into_iter()
            .map(|n| string_shape(source, n))
            .collect::<Option<Vec<_>>>()
            .map(StringAst::Dstr);
    }
    let parts = crate::cop::ported::ruby_string_parts(source, node)?;
    if parts.len() == 1 {
        Some(StringAst::Str(parts.into_iter().next()?))
    } else {
        Some(StringAst::Dstr(
            parts.into_iter().map(StringAst::Str).collect(),
        ))
    }
}
fn symbol_shape(source: &SourceFile, node: Node<'_>) -> Option<StringAst> {
    let value = crate::cop::ported::static_symbol(source, node)?;
    let parts = crate::cop::ported::ruby_string_parts(source, node);
    if let Some(parts) = parts.filter(|parts| parts.len() > 1) {
        Some(StringAst::Dstr(
            parts.into_iter().map(StringAst::Str).collect(),
        ))
    } else {
        Some(StringAst::Str(value))
    }
}
fn same_heredoc(source: &SourceFile, left: Node<'_>, right: Node<'_>, strict: bool) -> bool {
    let same_expression = |source: &SourceFile, left: Node<'_>, right: Node<'_>| {
        compare_expression(source, left, right, strict)
    };
    if left.kind() != "heredoc_beginning" || right.kind() != "heredoc_beginning" {
        return false;
    }
    let family = |n| {
        let text = node_text(source, n);
        (
            text.starts_with("<<~"),
            text.starts_with("<<-"),
            text.contains('\''),
            text.contains('`'),
        )
    };
    if family(left) != family(right) {
        return false;
    }
    let (Some((a_start, a_end)), Some((b_start, b_end))) = (
        crate::cop::ported::heredoc_span(source, left),
        crate::cop::ported::heredoc_span(source, right),
    ) else {
        return false;
    };
    if source.as_bytes()[a_start..a_end] != source.as_bytes()[b_start..b_end] {
        return false;
    }
    let mut root = left;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let a = interpolation_nodes(root, a_start, a_end);
    let b = interpolation_nodes(root, b_start, b_end);
    a.len() == b.len()
        && a.into_iter()
            .zip(b)
            .all(|(a, b)| same_expression(source, a, b))
}
fn interpolation_nodes<'a>(root: Node<'a>, start: usize, end: usize) -> Vec<Node<'a>> {
    let mut pending = vec![root];
    let mut nodes = Vec::new();
    while let Some(node) = pending.pop() {
        if node.kind() == "interpolation" && node.start_byte() >= start && node.end_byte() <= end {
            nodes.push(node);
        }
        pending.extend(code_children(node));
    }
    nodes.sort_by_key(Node::start_byte);
    nodes
}

fn float_value(source: &SourceFile, node: Node<'_>) -> Option<f64> {
    let (number, negative) = if node.kind() == "unary" {
        let operand = node.child_by_field_name("operand")?;
        if operand.kind() != "float" || !matches!(operator(source, node), Some(b"+" | b"-")) {
            return None;
        }
        (operand, operator(source, node) == Some(b"-"))
    } else if node.kind() == "float" {
        (node, false)
    } else {
        return None;
    };
    let value = node_text(source, number)
        .replace('_', "")
        .parse::<f64>()
        .ok()?;
    Some(if negative { -value } else { value })
}
#[derive(PartialEq)]
enum ComplexPart {
    Integer(num_bigint::BigInt),
    Float(f64),
    Rational(num_bigint::BigInt, num_bigint::BigInt),
}
fn complex_value(source: &SourceFile, node: Node<'_>) -> Option<ComplexPart> {
    let (number, negative) = if node.kind() == "unary" {
        let operand = node.child_by_field_name("operand")?;
        if operand.kind() != "complex" || !matches!(operator(source, node), Some(b"+" | b"-")) {
            return None;
        }
        (operand, operator(source, node) == Some(b"-"))
    } else if node.kind() == "complex" {
        (node, false)
    } else {
        return None;
    };
    let number = code_children(number).first().copied()?;
    if let Some(value) = integer_value(source, number) {
        return Some(ComplexPart::Integer(if negative { -value } else { value }));
    }
    if let Some(value) = float_value(source, number) {
        return Some(ComplexPart::Float(if negative { -value } else { value }));
    }
    let (value, denominator) = rational_value(source, number)?;
    Some(ComplexPart::Rational(
        if negative { -value } else { value },
        denominator,
    ))
}

/// Equality used by repeated legacy AST bindings.
pub fn same_expression(source: &SourceFile, left: Node<'_>, right: Node<'_>) -> bool {
    compare_expression(source, left, right, false)
}
/// Strict equality used before hash-based duplicate grouping.
pub fn eql_expression(source: &SourceFile, left: Node<'_>, right: Node<'_>) -> bool {
    compare_expression(source, left, right, true)
}
fn compare_expression(source: &SourceFile, left: Node<'_>, right: Node<'_>, strict: bool) -> bool {
    let same_expression = |source: &SourceFile, left: Node<'_>, right: Node<'_>| {
        compare_expression(source, left, right, strict)
    };
    if left == right {
        return true;
    }
    if let (Some(a), Some(b)) = (rational_value(source, left), rational_value(source, right)) {
        return a == b;
    }
    if let (Some(a), Some(b)) = (integer_value(source, left), integer_value(source, right)) {
        return a == b;
    }
    if let (Some(a), Some(b)) = (float_value(source, left), float_value(source, right)) {
        return a == b;
    }
    if let (Some(a), Some(b)) = (complex_value(source, left), complex_value(source, right)) {
        return complex_equal(&a, &b, strict);
    }
    let symbol_a = symbol_shape(source, left);
    let symbol_b = symbol_shape(source, right);
    if symbol_a.is_some() || symbol_b.is_some() {
        return symbol_a == symbol_b;
    }
    let string_a = string_shape(source, left);
    let string_b = string_shape(source, right);
    if string_a.is_some() || string_b.is_some() {
        return string_a == string_b;
    }
    if left.kind() == "heredoc_beginning" || right.kind() == "heredoc_beginning" {
        return same_heredoc(source, left, right, strict);
    }
    let send = |node| is_send(source, node);
    if send(left) && send(right) {
        let same_receiver = match (send_receiver(left), send_receiver(right)) {
            (None, None) => true,
            (Some(a), Some(b)) => same_expression(source, a, b),
            _ => false,
        };
        let same_block = match (call_block(left), call_block(right)) {
            (None, None) => true,
            (Some(a), Some(b)) => same_expression(source, a, b),
            _ => false,
        };
        return same_receiver
            && name(source, left) == name(source, right)
            && safe_send(source, left) == safe_send(source, right)
            && if strict {
                eql_arguments(source, &call_args(left), &call_args(right))
            } else {
                same_arguments(source, &call_args(left), &call_args(right))
            }
            && same_block;
    }
    if is_block(left) && is_block(right) {
        return same_block(source, left, right, strict);
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
        return same_regex(source, left, right, strict);
    }
    let a = code_children(left);
    let b = code_children(right);
    if a.is_empty()
        && b.is_empty()
        && matches!(
            left.kind(),
            "array"
                | "hash"
                | "parenthesized_statements"
                | "block_parameters"
                | "body_statement"
                | "block_body"
        )
    {
        return true;
    }
    if a.is_empty() || b.is_empty() {
        return node_bytes(source, left) == node_bytes(source, right);
    }
    if let (Some(a), Some(b)) = (operator(source, left), operator(source, right)) {
        if a != b {
            return false;
        }
    }
    a.len() == b.len()
        && a.iter()
            .zip(&b)
            .all(|(a, b)| same_expression(source, *a, *b))
}

pub fn same_arguments(source: &SourceFile, left: &[Node<'_>], right: &[Node<'_>]) -> bool {
    compare_arguments(source, left, right, false)
}
pub fn eql_arguments(source: &SourceFile, left: &[Node<'_>], right: &[Node<'_>]) -> bool {
    compare_arguments(source, left, right, true)
}
fn compare_arguments(
    source: &SourceFile,
    left: &[Node<'_>],
    right: &[Node<'_>],
    strict: bool,
) -> bool {
    let same_expression = |source: &SourceFile, left: Node<'_>, right: Node<'_>| {
        compare_expression(source, left, right, strict)
    };
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

fn complex_equal(left: &ComplexPart, right: &ComplexPart, strict: bool) -> bool {
    if strict || left == right {
        return left == right;
    }
    match (left, right) {
        (ComplexPart::Float(a), ComplexPart::Float(b)) => a == b,
        (ComplexPart::Float(a), ComplexPart::Integer(b))
        | (ComplexPart::Integer(b), ComplexPart::Float(a)) => integer_float_equal(b, *a),
        (ComplexPart::Float(a), ComplexPart::Rational(n, d))
        | (ComplexPart::Rational(n, d), ComplexPart::Float(a)) => rational_float(n, d) == Some(*a),
        (ComplexPart::Integer(a), ComplexPart::Rational(b, d))
        | (ComplexPart::Rational(b, d), ComplexPart::Integer(a)) => a * d == *b,
        _ => false,
    }
}
fn integer_float_equal(integer: &num_bigint::BigInt, float: f64) -> bool {
    use num_bigint::BigInt;
    if !float.is_finite() {
        return false;
    }
    let bits = float.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mut mantissa = bits & ((1u64 << 52) - 1);
    if exponent != 0 {
        mantissa |= 1u64 << 52;
    }
    let shift = if exponent == 0 {
        -1074
    } else {
        exponent - 1023 - 52
    };
    let value = BigInt::from(mantissa);
    let value = if shift >= 0 {
        value << (shift as usize)
    } else {
        let divisor = BigInt::from(1u8) << ((-shift) as usize);
        if &value % &divisor != BigInt::from(0u8) {
            return false;
        }
        value / divisor
    };
    *integer == if bits >> 63 != 0 { -value } else { value }
}

fn rational_float(n: &num_bigint::BigInt, d: &num_bigint::BigInt) -> Option<f64> {
    Some(n.to_string().parse::<f64>().ok()? / d.to_string().parse::<f64>().ok()?)
}

fn rational_value(
    source: &SourceFile,
    node: Node<'_>,
) -> Option<(num_bigint::BigInt, num_bigint::BigInt)> {
    use num_bigint::BigInt;
    if node.kind() == "unary" {
        let operand = node.child_by_field_name("operand")?;
        if operand.kind() != "rational" {
            return None;
        }
        let (value, denominator) = rational_value(source, operand)?;
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
fn regex_parts<'a>(
    source: &SourceFile,
    node: Node<'a>,
    ruby_version: f64,
) -> Option<(Vec<RegexPart<'a>>, Vec<u8>)> {
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
                let text = crate::cop::ported::decode_regexp_content(
                    &content,
                    opening,
                    closing,
                    ruby_version,
                )?;
                if !text.is_empty() {
                    parts.push(RegexPart::Text(text));
                }
                content.clear();
            }
            parts.push(RegexPart::Expression(child));
        } else {
            content.extend_from_slice(node_bytes(source, child));
        }
    }
    if !content.is_empty() {
        let text =
            crate::cop::ported::decode_regexp_content(&content, opening, closing, ruby_version)?;
        if !text.is_empty() {
            parts.push(RegexPart::Text(text));
        }
    }
    let last = node.child(node.child_count().checked_sub(1)?.try_into().ok()?)?;
    let mut options = node_bytes(source, last).get(1..)?.to_vec();
    options.sort_unstable();
    options.dedup();
    Some((parts, options))
}

pub fn static_regex_value_for_ruby(
    source: &SourceFile,
    node: Node<'_>,
    ruby_version: f64,
) -> Option<String> {
    let (mut parts, options) = regex_parts(source, node, ruby_version)?;
    if !options.is_empty() || parts.len() != 1 {
        return None;
    }
    match parts.pop()? {
        RegexPart::Text(bytes) => String::from_utf8(bytes).ok(),
        RegexPart::Expression(_) => None,
    }
}

fn same_regex(source: &SourceFile, left: Node<'_>, right: Node<'_>, strict: bool) -> bool {
    let same_expression = |source: &SourceFile, left: Node<'_>, right: Node<'_>| {
        compare_expression(source, left, right, strict)
    };
    let (Some((a, flags_a)), Some((b, flags_b))) = (
        regex_parts(source, left, 3.4),
        regex_parts(source, right, 3.4),
    ) else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct OriginalComparison {
        source: String,
        same: bool,
        eql: bool,
    }
    #[test]
    fn regex_values_match_original_parser_versions() {
        #[derive(serde::Deserialize)]
        struct Case {
            source: String,
            ruby_version: f64,
            value: Option<String>,
        }
        let cases: Vec<Case> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/original-regexp-static-value.json"
        )))
        .unwrap();
        for case in cases {
            let source = SourceFile::from_bytes("regexp.rb", case.source.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            let node = tree.root_node().named_child(0).unwrap();
            assert_eq!(
                static_regex_value_for_ruby(&source, node, case.ruby_version),
                case.value,
                "Ruby {}: {}",
                case.ruby_version,
                case.source
            );
        }
    }
    #[test]
    fn comparisons_match_original_ruby_ast_equalities() {
        let cases: Vec<OriginalComparison> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/framework-expression-equality.json"
        )))
        .unwrap();
        for case in cases {
            let source = SourceFile::from_bytes("expression.rb", case.source.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(!tree.root_node().has_error(), "{}", case.source);
            let array = tree.root_node().named_child(0).unwrap();
            let nodes = code_children(array);
            assert_eq!(nodes.len(), 2, "{}", case.source);
            assert_eq!(
                same_expression(&source, nodes[0], nodes[1]),
                case.same,
                "Node#==: {}",
                case.source
            );
            assert_eq!(
                eql_expression(&source, nodes[0], nodes[1]),
                case.eql,
                "Node#eql?: {}",
                case.source
            );
        }
    }
}
