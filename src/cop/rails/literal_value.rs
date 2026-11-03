use crate::cop::shared::ruby_expression::{float_value, integer_ast, plain_symbol, rational_value};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn float_text(value: f64) -> String {
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-Infinity"
        } else {
            "Infinity"
        }
        .into();
    }
    if value != 0.0 && (value.abs() < 0.0001 || value.abs() >= 1e15) {
        let text = format!("{value:e}");
        let (mantissa, exponent) = text.split_once('e').unwrap();
        let mantissa = if mantissa.contains('.') {
            mantissa.to_owned()
        } else {
            format!("{mantissa}.0")
        };
        return format!("{mantissa}e{:+03}", exponent.parse::<i32>().unwrap());
    }
    format!("{value:?}")
}

pub(super) fn value(source: &SourceFile, node: Node<'_>) -> Option<String> {
    plain_symbol(source, node)
        .or_else(|| crate::cop::ported::original_string_value(source, node))
        .or_else(|| integer_ast(source, node).map(|n| n.to_string()))
        .or_else(|| float_value(source, node).map(float_text))
        .or_else(|| rational_value(source, node).map(|(n, d)| format!("{n}/{d}")))
        .or_else(|| super::literal_ast_value::command_value(source, node))
}
