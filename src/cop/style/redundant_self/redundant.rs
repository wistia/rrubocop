//! Detect whether `self.` is redundant on a call.

use tree_sitter::Node;

use crate::cop::shared::call_method_name;
use crate::parse::source::SourceFile;

use super::shadow::name_is_shadowed;

pub(super) fn self_is_redundant(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(method) = call_method_name(source, node) else {
        return false;
    };
    if excluded_method(method) {
        return false;
    }
    !name_is_shadowed(source, node, method)
}

fn excluded_method(method: &[u8]) -> bool {
    matches!(
        method,
        b"class"
            | b"select"
            | b"for"
            | b"in"
            | b"if"
            | b"unless"
            | b"while"
            | b"until"
            | b"case"
            | b"when"
            | b"then"
            | b"else"
            | b"elsif"
            | b"begin"
            | b"end"
            | b"rescue"
            | b"ensure"
            | b"return"
            | b"yield"
            | b"super"
            | b"defined?"
            | b"not"
            | b"and"
            | b"or"
    ) || method.starts_with(b"[")
        || method.first().is_some_and(|b| b.is_ascii_uppercase())
        || is_setter_name(method)
        || !is_simple_method_name(method)
}

fn is_setter_name(method: &[u8]) -> bool {
    method.ends_with(b"=")
        && method != b"=="
        && method != b"!="
        && method != b"=~"
        && method != b"!~"
}

fn is_simple_method_name(method: &[u8]) -> bool {
    method
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'_' | b'!' | b'?' | b'='))
}
