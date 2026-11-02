//! Style/ExactRegexpMatch: the original anchored literal-token predicate.
use crate::cop::shared::{
    call_method_name, call_receiver, node_bytes, push_replace, semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ExactRegexpMatch;

fn interval(bytes: &[u8]) -> bool {
    let Some(end) = bytes.iter().position(|b| *b == b'}') else {
        return false;
    };
    let inner = &bytes[1..end];
    if inner.is_empty() {
        return false;
    }
    if let Some(comma) = inner.iter().position(|b| *b == b',') {
        (comma > 0 || comma + 1 < inner.len())
            && inner[..comma].iter().all(u8::is_ascii_digit)
            && inner[comma + 1..].iter().all(u8::is_ascii_digit)
    } else {
        inner.iter().all(u8::is_ascii_digit)
    }
}
fn literal(pattern: &str) -> Option<(&str, bool)> {
    let rest = pattern.strip_prefix("\\A")?;
    let bytes = rest.as_bytes();
    let end = if bytes.first() == Some(&b'\\') {
        let second = *bytes.get(1)?;
        let literal_escape = matches!(second,
            b'\0'..=b'#' | b'%'..=b'\'' | b','..=b'-' | b'/' | b':'..=b'>' | b'@' | b'_'..=b'`' | b'~'..=b'\x7f')
            || b"EFIJLNOQTUVYgijklmoqy".contains(&second);
        if !literal_escape {
            return None;
        }
        2
    } else {
        let mut end = 0;
        while end < bytes.len() {
            let b = bytes[end];
            if b".()|*+?[^$\\".contains(&b) || b == b'{' && interval(&bytes[end..]) {
                break;
            }
            end += 1;
        }
        end
    };
    if end == 0 {
        return None;
    }
    let tail = rest.get(end..)?.strip_prefix("\\z")?;
    Some((&rest[..end], tail.is_empty()))
}
fn observed(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "program" => false,
        "body_statement" | "block_body" | "then" | "else" | "parenthesized_statements" => {
            let mut cursor = parent.walk();
            let last = parent
                .named_children(&mut cursor)
                .filter(|n| n.kind() != "comment")
                .last();
            last == Some(node) && observed(parent)
        }
        _ => true,
    }
}
fn captures_observed(source: &SourceFile, mut root: Node<'_>) -> bool {
    while let Some(parent) = root.parent() {
        root = parent;
        if matches!(
            root.kind(),
            "method" | "singleton_method" | "class" | "module" | "singleton_class"
        ) {
            break;
        }
    }
    fn scan(source: &SourceFile, node: Node<'_>, root: Node<'_>) -> bool {
        if node != root
            && matches!(
                node.kind(),
                "method" | "singleton_method" | "class" | "module" | "singleton_class"
            )
        {
            return false;
        }
        if node.kind() == "global_variable" {
            let text = node_bytes(source, node);
            if matches!(text, b"$~" | b"$&" | b"$+" | b"$'" | b"$`")
                || text.get(1).is_some_and(u8::is_ascii_digit)
            {
                return true;
            }
        }
        if node.kind() == "call"
            && call_method_name(source, node) == Some(b"last_match")
            && call_receiver(node).is_some_and(|n| node_bytes(source, n) == b"Regexp")
        {
            return true;
        }
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .any(|child| scan(source, child, root))
    }
    scan(source, root, root)
}
impl Cop for ExactRegexpMatch {
    fn name(&self) -> &'static str {
        "Style/ExactRegexpMatch"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let (receiver, method, regexp) = if node.kind() == "binary" {
            let Some(op) = node.child_by_field_name("operator") else {
                return;
            };
            let (Some(left), Some(right)) = (
                node.child_by_field_name("left"),
                node.child_by_field_name("right"),
            ) else {
                return;
            };
            (left, node_bytes(source, op), right)
        } else {
            let (Some(receiver), Some(method)) =
                (call_receiver(node), call_method_name(source, node))
            else {
                return;
            };
            let args = semantic_argument_nodes(node);
            if args.len() != 1 {
                return;
            }
            (receiver, method, args[0])
        };
        if !matches!(method, b"=~" | b"===" | b"!~" | b"match" | b"match?")
            || regexp.kind() != "regex"
        {
            return;
        }
        let Some(pattern) = crate::cop::shared::ruby_expression::static_regex_value(source, regexp)
        else {
            return;
        };
        let Some((literal, entire)) = literal(&pattern) else {
            return;
        };
        let new_method = if method == b"!~" { "!=" } else { "==" };
        let prefer = format!(
            "{} {new_method} '{literal}'",
            String::from_utf8_lossy(node_bytes(source, receiver))
        );
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(source, line, col, format!("Use `{prefer}`."));
        let mut end = node
            .child_by_field_name("block")
            .map_or(node.end_byte(), |n| n.start_byte());
        while end > node.start_byte() && source.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                .chars()
                .count();
        let safe_nav = node
            .child_by_field_name("operator")
            .is_some_and(|n| node_bytes(source, n) == b"&.");
        let changes_result = matches!(method, b"=~" | b"match" | b"===") || safe_nav;
        let comments = !crate::cop::shared::collect_comments(node).is_empty();
        let safe = entire
            && !literal.contains(['\'', '\\'])
            && !(changes_result && observed(node))
            && !(matches!(method, b"=~" | b"!~" | b"match") && captures_observed(source, node))
            && node.child_by_field_name("block").is_none()
            && !comments
            && std::str::from_utf8(node_bytes(source, receiver)).is_ok();
        if safe {
            diagnostic.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                node.end_byte(),
                prefer,
                self.name(),
            );
        } else {
            diagnostic.correctable = false;
        }
        diagnostics.push(diagnostic);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_diagnostics_with_safety_guards() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/exact_regexp_match/intentional-safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&ExactRegexpMatch, &cases.to_string());
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &ExactRegexpMatch,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/exact_regexp_match/parity.json"
            )),
        );
    }
}
