use super::*;
use std::borrow::Cow;

#[test]
fn static_values_decode_quoted_percent_and_concatenated_literals() {
    for (literal, expected) in [
        (":name", Some("name")),
        ("%s(name)", Some("name")),
        (r#":"na\x6de""#, Some("name")),
        (r#""na\u006de""#, Some("name")),
        (r#"%Q(na\155e)"#, Some("name")),
        (r#"%q(na\x6de)"#, Some(r"na\x6de")),
        (r#""na" "me""#, Some("name")),
        (r#":"\cM""#, Some("\r")),
        (r#""\C-M""#, Some("\r")),
        (r##""#{VALUE}""##, None),
        (r##":"#{VALUE}""##, None),
        (r#"%Q(#{value})"#, None),
        ("'#{VALUE}'", Some("#{VALUE}")),
    ] {
        assert_literal(literal, expected);
    }
}

fn assert_literal(literal: &str, expected: Option<&str>) {
    let source = SourceFile::from_bytes("literal.rb", format!("value({literal})\n").into_bytes());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    assert!(!tree.root_node().has_error(), "{literal}");
    let node = args(tree.root_node().named_child(0).unwrap())[0];
    let config = CopConfig::default();
    assert_eq!(
        Context::new(&source, &config).static_value(node).as_deref(),
        expected,
        "{literal}"
    );
}

#[test]
fn plain_literal_and_constant_names_borrow_source() {
    let source = SourceFile::from_bytes(
        "literal.rb",
        b"value(:name, \"name\", A::B, ::A)\n".to_vec(),
    );
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let arguments = args(tree.root_node().named_child(0).unwrap());
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    assert!(matches!(
        c.static_symbol(arguments[0]),
        Some(Cow::Borrowed("name"))
    ));
    assert!(matches!(
        c.static_string(arguments[1]),
        Some(Cow::Borrowed("name"))
    ));
    assert!(matches!(
        c.constant_name(arguments[2]),
        Some(Cow::Borrowed("A::B"))
    ));
    assert!(matches!(
        c.constant_name(arguments[3]),
        Some(Cow::Borrowed("A"))
    ));
}

#[test]
fn constant_names_ignore_formatting_but_reject_dynamic_scopes() {
    for (text, expected) in [
        ("A::\n # namespace\n B", Some("A::B")),
        ("::\n A::B", Some("A::B")),
        ("::A::\n B::C", Some("A::B::C")),
        ("container::B", None),
    ] {
        assert_constant(text, expected);
    }
}

fn assert_constant(text: &str, expected: Option<&str>) {
    let source = SourceFile::from_bytes("constant.rb", format!("value({text})\n").into_bytes());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    assert!(!tree.root_node().has_error(), "{text}");
    let node = args(tree.root_node().named_child(0).unwrap())[0];
    let config = CopConfig::default();
    assert_eq!(
        Context::new(&source, &config)
            .constant_name(node)
            .as_deref(),
        expected,
        "{text}"
    );
}
