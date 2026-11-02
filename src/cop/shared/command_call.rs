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
