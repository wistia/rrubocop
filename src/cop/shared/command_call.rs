use super::{call_receiver, is_setter_call, method_node, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

/// A unary argument sign misrepresented as binary by tree-sitter; not a complete call projection.
pub struct CommandUnaryPlus<'a> {
    pub operator: Node<'a>,
    pub selector: Node<'a>,
    pub receiver: Option<Node<'a>>,
    pub right: Node<'a>,
}

fn spaced(source: &SourceFile, start: usize, end: usize) -> bool {
    let Some(gap) = source.as_bytes().get(start..end) else {
        return false;
    };
    let mut bytes = gap.iter().copied().peekable();
    let mut horizontal = false;
    while let Some(byte) = bytes.next() {
        match byte {
            b' ' | b'\t' | b'\x0b' | b'\x0c' | b'\r' => horizontal = true,
            b'\\' if bytes.peek() == Some(&b'\n') => {
                bytes.next();
                horizontal = true;
            }
            b'\\' if bytes.peek() == Some(&b'\r') => {
                bytes.next();
                if bytes.next() != Some(b'\n') {
                    return false;
                }
                horizontal = true;
            }
            _ => return false,
        }
    }
    horizontal
}

pub fn command_unary_plus<'a>(source: &SourceFile, node: Node<'a>) -> Option<CommandUnaryPlus<'a>> {
    if node.kind() != "binary" {
        return None;
    }
    let operator = node.child_by_field_name("operator")?;
    if node_bytes(source, operator) != b"+"
        || source
            .as_bytes()
            .get(operator.end_byte())
            .is_none_or(u8::is_ascii_whitespace)
    {
        return None;
    }
    let left = node.child_by_field_name("left")?;
    if !spaced(source, left.end_byte(), operator.start_byte()) {
        return None;
    }
    let mut callable = left;
    while callable.kind() == "unary" {
        callable = callable.child_by_field_name("operand")?;
    }
    let (selector, receiver) = match callable.kind() {
        "identifier" if crate::cop::ported::bare_call_identifier(source, callable) => {
            (callable, None)
        }
        "call"
            if callable.child_by_field_name("arguments").is_none()
                && callable.child_by_field_name("block").is_none()
                && !is_setter_call(callable) =>
        {
            (method_node(callable)?, call_receiver(callable))
        }
        _ => return None,
    };
    Some(CommandUnaryPlus {
        operator,
        selector,
        receiver,
        right: node.child_by_field_name("right")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(serde::Deserialize)]
    struct Token {
        r#type: String,
        source: String,
        position: [usize; 2],
    }
    #[derive(serde::Deserialize)]
    struct Case {
        source: String,
        ruby_version: f64,
        tokens: Vec<Token>,
    }
    #[test]
    fn unary_plus_roles_match_original_lexer() {
        let rows: Vec<Case> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/original-command-unary-plus.json"
        )))
        .unwrap();
        for case in rows {
            let source = SourceFile::from_bytes("command.rb", case.source.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            fn check(source: &SourceFile, node: Node<'_>, case: &Case) {
                if node.kind() == "binary" {
                    if let Some(operator) = node
                        .child_by_field_name("operator")
                        .filter(|n| node_bytes(source, *n) == b"+")
                    {
                        let (line, column) = source.offset_to_line_col(operator.start_byte());
                        let token = case
                            .tokens
                            .iter()
                            .find(|t| t.position == [line, column] && t.source == "+")
                            .unwrap_or_else(|| {
                                panic!(
                                    "Original token unavailable: {} at {line}:{column}",
                                    case.source
                                )
                            });
                        let expected = matches!(token.r#type.as_str(), "tUNARY_NUM" | "tUPLUS");
                        assert_eq!(
                            command_unary_plus(source, node).is_some(),
                            expected,
                            "Ruby{}: {} at {line}:{column} ({})",
                            case.ruby_version,
                            case.source,
                            token.r#type
                        );
                    }
                }
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    check(source, child, case);
                }
            }
            check(&source, tree.root_node(), &case);
        }
    }
}

pub struct CommandArgument<'a> {
    pub node: Node<'a>,
    pub start: usize,
    pub end: usize,
    pub unary_plus: bool,
}

pub struct UnaryCommandCall<'a> {
    pub selector: Node<'a>,
    pub receiver: Option<Node<'a>>,
    pub start: usize,
    pub end: usize,
    pub arguments: Vec<CommandArgument<'a>>,
}

fn argument_root(mut node: Node<'_>) -> Node<'_> {
    while let Some(parent) = node.parent() {
        let extends = match parent.kind() {
            "binary" => {
                parent.child_by_field_name("left") == Some(node)
                    && parent
                        .child_by_field_name("operator")
                        .is_some_and(|op| !matches!(op.kind(), "and" | "or"))
            }
            "range" => parent.child_by_field_name("begin") == Some(node),
            "conditional" => parent.child_by_field_name("condition") == Some(node),
            _ => false,
        };
        if !extends {
            break;
        }
        node = parent;
    }
    node
}

fn command_context(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "program"
            | "body_statement"
            | "block_body"
            | "then"
            | "else"
            | "do"
            | "parenthesized_statements"
            | "interpolation" => return true,
            "argument_list" => {
                return super::named_kids(parent)
                    .into_iter()
                    .find(|n| n.kind() != "comment")
                    == Some(node);
            }
            "array" | "hash" | "pair" | "pattern" | "optional_parameter" | "keyword_parameter"
            | "method_parameters" | "lambda_parameters" => return false,
            "unary"
                if parent
                    .child_by_field_name("operator")
                    .is_some_and(|op| op.kind() == "defined?") =>
            {
                return false;
            }
            "binary"
            | "range"
            | "conditional"
            | "unary"
            | "assignment"
            | "operator_assignment"
            | "rescue_modifier"
            | "if"
            | "unless"
            | "while"
            | "until"
            | "if_modifier"
            | "unless_modifier"
            | "while_modifier"
            | "until_modifier"
            | "case"
            | "for"
            | "in" => (),
            _ => return false,
        }
        node = parent;
    }
    false
}

pub fn command_unary_plus_call<'a>(
    source: &SourceFile,
    node: Node<'a>,
) -> Option<UnaryCommandCall<'a>> {
    let sign = command_unary_plus(source, node)?;
    if node.has_error() {
        return None;
    }
    let mut left = node.child_by_field_name("left")?;
    if left.kind() == "unary" {
        if !left
            .child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"!")
        {
            return None;
        }
        left = left.child_by_field_name("operand")?;
        if left.kind() == "unary" {
            return None;
        }
    }
    let expression = argument_root(node);
    if expression.has_error() || !command_context(expression) {
        return None;
    }
    let argument = if expression == node {
        sign.right
    } else {
        expression
    };
    let mut arguments = vec![CommandArgument {
        node: argument,
        start: sign.operator.start_byte(),
        end: expression.end_byte(),
        unary_plus: expression == node,
    }];
    if let Some(parent) = expression.parent().filter(|p| p.kind() == "argument_list") {
        if parent.has_error() {
            return None;
        }
        for sibling in super::named_kids(parent)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .skip(1)
        {
            arguments.push(CommandArgument {
                node: sibling,
                start: sibling.start_byte(),
                end: sibling.end_byte(),
                unary_plus: false,
            });
        }
    }
    let start = sign
        .receiver
        .map_or(sign.selector.start_byte(), |n| n.start_byte());
    let end = arguments.last()?.end;
    Some(UnaryCommandCall {
        selector: sign.selector,
        receiver: sign.receiver,
        start,
        end,
        arguments,
    })
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    #[derive(serde::Deserialize, Debug, PartialEq, Eq)]
    struct Argument {
        source: String,
    }
    #[derive(serde::Deserialize, Debug, PartialEq, Eq)]
    struct Call {
        selector: String,
        source: String,
        position: [usize; 2],
        receiver: Option<String>,
        arguments: Vec<Argument>,
    }
    #[derive(serde::Deserialize)]
    struct Case {
        source: String,
        ruby_version: f64,
        valid_syntax: bool,
        calls: Vec<Call>,
    }
    #[test]
    fn command_bounds_match_original_ast() {
        let rows: Vec<Case> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/original-command-unary-plus.json"
        )))
        .unwrap();
        for case in rows.into_iter().filter(|c| c.valid_syntax) {
            let source = SourceFile::from_bytes("command.rb", case.source.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            if tree.root_node().has_error() {
                continue;
            }
            fn collect(source: &SourceFile, node: Node<'_>, calls: &mut Vec<Call>) {
                if let Some(call) = command_unary_plus_call(source, node) {
                    let (line, column) = source.offset_to_line_col(call.selector.start_byte());
                    let text = |start, end| {
                        String::from_utf8_lossy(&source.as_bytes()[start..end])
                            .replace("\r\n", "\n")
                    };
                    calls.push(Call {
                        selector: super::super::node_text(source, call.selector),
                        source: text(call.start, call.end),
                        position: [line, column],
                        receiver: call.receiver.map(|n| super::super::node_text(source, n)),
                        arguments: call
                            .arguments
                            .iter()
                            .map(|a| Argument {
                                source: text(a.start, a.end),
                            })
                            .collect(),
                    });
                }
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    collect(source, child, calls);
                }
            }
            let mut actual = Vec::new();
            collect(&source, tree.root_node(), &mut actual);
            assert_eq!(
                actual, case.calls,
                "Ruby {}: {}",
                case.ruby_version, case.source
            );
        }
    }
}
