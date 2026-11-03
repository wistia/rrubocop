use super::{call_method_name, call_receiver, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;
fn regexp_constant(source: &SourceFile, mut node: Node<'_>) -> bool {
    while node.kind() == "parenthesized_statements" {
        let parts: Vec<_> = super::named_kids(node)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .collect();
        if parts.len() != 1 {
            return false;
        }
        node = parts[0];
    }
    if node.kind() == "scope_resolution" && node.child_by_field_name("scope").is_none() {
        let Some(name) = node.child_by_field_name("name") else {
            return false;
        };
        node = name;
    }
    node.kind() == "constant" && node_bytes(source, node) == b"Regexp"
}

pub fn indirect_capture_observer(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(
        call_method_name(source, node),
        Some(b"send" | b"public_send" | b"__send__" | b"method")
    ) && call_receiver(node).is_some_and(|n| regexp_constant(source, n))
        && super::argument_nodes(node)
            .first()
            .and_then(|n| crate::cop::ported::static_symbol(source, *n))
            .as_deref()
            == Some("last_match")
}

pub fn captures_observed(source: &SourceFile, mut root: Node<'_>) -> bool {
    while let Some(parent) = root.parent() {
        root = parent;
        if matches!(
            root.kind(),
            "method" | "singleton_method" | "class" | "module" | "singleton_class"
        ) {
            break;
        }
    }
    struct Captures(std::sync::Mutex<std::collections::HashMap<(usize, usize, u16), bool>>);
    let cached = source.cached(0, || Captures(Default::default()));
    let mut observations = cached.0.lock().unwrap();
    *observations
        .entry((root.start_byte(), root.end_byte(), root.kind_id()))
        .or_insert_with(|| scan(source, root, root))
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
        if matches!(
            text,
            b"$~"
                | b"$&"
                | b"$+"
                | b"$'"
                | b"$`"
                | b"$MATCH"
                | b"$PREMATCH"
                | b"$POSTMATCH"
                | b"$LAST_PAREN_MATCH"
                | b"$LAST_MATCH_INFO"
        ) || text.get(1).is_some_and(u8::is_ascii_digit)
        {
            return true;
        }
    }
    if node.kind() == "call"
        && call_method_name(source, node) == Some(b"last_match")
        && call_receiver(node).is_some_and(|n| regexp_constant(source, n))
    {
        return true;
    }
    if indirect_capture_observer(source, node) {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| scan(source, child, root))
}

use std::sync::LazyLock;
static DETERMINISTIC: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"^(?:[a-zA-Z0-9_\t\n\r\x0b\x0c \-,"'!#%&<>=;:`~/]|\\[^AbBdDgGhHkpPRwWXsSzZ0-9])+$"#,
    )
    .unwrap()
});

/// The pinned original Util::LITERAL_REGEX, repeated over a nonempty complete string.
pub fn is_literal_regexp_text(text: &str) -> bool {
    DETERMINISTIC.is_match(text)
}

/// Ruby double-quoted escape decoding; unlike StringInterpreter, includes control escapes.
pub fn decode_ruby_double_quoted_content(text: &str) -> Option<String> {
    crate::cop::ported::decode_ruby_double_quoted_content(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_capture_facts_preserve_exact_scope_scans_and_snapshot_lifetimes() {
        for code in [
            "first.split(/x/); second.split(/y/)",
            "first.split(/x/); $1; def separate; second.split(/y/); end",
            "def outer; first.split(/x/); items.each { Regexp.last_match }; end; second.split(/y/)",
            "class A; first.split(/x/); def nested; second.split(/y/); $~; end; end",
            "module A; first.split(/x/); class B; Regexp.public_send(:last_match); end; end",
            "class << self; first.split(/x/); Regexp.method(:last_match).call; end",
            "first.split(/x/); Other::Regexp.last_match; ::Regexp2.last_match",
        ] {
            let source = SourceFile::from_bytes("capture.rb", code.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(!tree.root_node().has_error(), "{code}");
            super::super::for_each_descendant(tree.root_node(), |node| {
                if node.kind() != "call" {
                    return;
                }
                let mut scope = node;
                while let Some(parent) = scope.parent() {
                    scope = parent;
                    if matches!(
                        scope.kind(),
                        "method" | "singleton_method" | "class" | "module" | "singleton_class"
                    ) {
                        break;
                    }
                }
                let expected = scan(&source, scope, scope);
                assert_eq!(captures_observed(&source, node), expected, "{code}");
                assert_eq!(captures_observed(&source, node), expected, "cached {code}");
            });
        }
        for (code, expected) in [("value.split(/x/); $1", true), ("value.split(/x/)", false)] {
            let source = SourceFile::from_bytes("same.rb", code.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert_eq!(
                captures_observed(&source, tree.root_node().named_child(0).unwrap()),
                expected
            );
        }
    }

    #[test]
    fn capture_observers_resolve_only_the_root_regexp_constant() {
        for (receiver, expected) in [
            ("Regexp", true),
            ("::Regexp", true),
            (":: Regexp", true),
            ("(Regexp)", true),
            ("((:: Regexp))", true),
            ("Other::Regexp", false),
            ("Regexp2", false),
            ("regexp", false),
        ] {
            for suffix in [
                "last_match",
                "public_send(:last_match)",
                "__send__(:last_match)",
                "method(:last_match).call",
            ] {
                let code = format!("string.partition(/f/); {receiver}.{suffix}");
                let source = SourceFile::from_bytes("capture.rb", code.as_bytes().to_vec());
                let tree = crate::parse::parse_ruby(&source).unwrap();
                assert!(!tree.root_node().has_error(), "{code}");
                let call = tree.root_node().named_child(0).unwrap();
                assert_eq!(
                    captures_observed(&source, call),
                    expected,
                    "{receiver}.{suffix}"
                );
            }
        }
    }
    #[test]
    fn original_literal_regex_predicate() {
        let cases: Vec<(String, bool)> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/original-literal-regexp-predicate.json"
        )))
        .unwrap();
        for (text, expected) in cases {
            assert_eq!(is_literal_regexp_text(&text), expected, "{text:?}");
        }
    }
}
