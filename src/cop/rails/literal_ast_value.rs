use crate::cop::shared::ruby_expression::{
    integer_ast, plain_string, plain_symbol, regular_send_arguments, regular_send_name,
    regular_send_receiver,
};
use crate::cop::shared::ruby_literal::{string_inspect, symbol_inspect};
use crate::cop::shared::{named_kids, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn sexp(source: &SourceFile, node: Node<'_>, indent: usize) -> Option<String> {
    let pad = "  ".repeat(indent);
    if let Some(value) = plain_string(source, node) {
        return Some(format!("{pad}(str {})", string_inspect(&value)));
    }
    if let Some(value) = plain_symbol(source, node) {
        return Some(format!("{pad}(sym {})", symbol_inspect(&value)));
    }
    if let Some(value) = integer_ast(source, node) {
        return Some(format!("{pad}(int {value})"));
    }
    if matches!(node.kind(), "true" | "false" | "nil" | "self") {
        return Some(format!("{pad}({})", node.kind()));
    }
    if node.kind() == "identifier" && crate::cop::ported::local_identifier(source, node) {
        return Some(format!(
            "{pad}(lvar {})",
            symbol_inspect(std::str::from_utf8(node_bytes(source, node)).ok()?)
        ));
    }
    if let Some(method) = regular_send_name(source, node) {
        let mut result = format!("{pad}(send");
        if let Some(receiver) = regular_send_receiver(source, node).flatten() {
            result.push('\n');
            result.push_str(&sexp(source, receiver, indent + 1)?);
        } else {
            result.push_str(" nil");
        }
        result.push(' ');
        result.push_str(&symbol_inspect(std::str::from_utf8(&method).ok()?));
        for arg in regular_send_arguments(source, node)? {
            result.push('\n');
            result.push_str(&sexp(source, arg, indent + 1)?);
        }
        result.push(')');
        return Some(result);
    }
    None
}
pub(super) fn command_value(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if node.kind() != "subshell" {
        return None;
    }
    let children = named_kids(node);
    let Some(first) = children.first() else {
        return Some(String::new());
    };
    if first.kind() == "interpolation" {
        let mut result = String::from("(begin");
        for child in named_kids(*first) {
            result.push('\n');
            result.push_str(&sexp(source, child, 1)?);
        }
        result.push(')');
        return Some(result);
    }
    let end = children
        .iter()
        .find(|n| n.kind() == "interpolation")
        .map_or(node.end_byte() - 1, |n| n.start_byte());
    let prefix = std::str::from_utf8(source.as_bytes().get(first.start_byte()..end)?).ok()?;
    let delimiter = ['|', '~', '!', '^', ';']
        .into_iter()
        .find(|delimiter| !prefix.contains(*delimiter))?;
    let text = format!("%Q{delimiter}{prefix}{delimiter}");
    let synthetic = SourceFile::from_bytes("literal.rb", text.into_bytes());
    let tree = crate::parse::parse_ruby(&synthetic).ok()?;
    let literal = tree.root_node().named_child(0)?;
    let parts = crate::cop::ported::ruby_string_parts(&synthetic, literal)?;
    Some(format!("(str {})", string_inspect(parts.first()?)))
}
