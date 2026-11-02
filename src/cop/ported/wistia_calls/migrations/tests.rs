use super::*;
use crate::cop::CopConfig;
use crate::parse::source::SourceFile;

fn findings(code: &str, key: &str) -> Vec<Finding> {
    let source = SourceFile::from_bytes("db/migrate/probe.rb", code.as_bytes().to_vec());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    table_option(&c, candidate(&c, tree.root_node()), key, "test_value")
}
fn inspect(code: &str, key: &str) -> Finding {
    findings(code, key).remove(0)
}

fn candidate<'n>(c: &Context<'_>, n: Node<'n>) -> Node<'n> {
    descendants(n)
        .find(|n| is_call(*n) && c.method(*n) == "create_table")
        .unwrap()
}

fn corrected(code: &str, key: &str) -> String {
    let (start, end, text) = inspect(code, key).edit.unwrap();
    let mut result = code.to_owned();
    result.replace_range(start..end, &text);
    result
}

#[test]
fn table_options_are_inserted_inside_actual_argument_lists() {
    for (source, expected) in [
        (
            "create_table(:widgets)\n",
            "create_table(:widgets, charset: \"test_value\")\n",
        ),
        (
            "create_table :widgets\n",
            "create_table :widgets, charset: \"test_value\"\n",
        ),
        (
            "create_table(:widgets,)\n",
            "create_table(:widgets, charset: \"test_value\",)\n",
        ),
        (
            "create_table(:widgets) do |t|\n t.string :name\nend\n",
            "create_table(:widgets, charset: \"test_value\") do |t|\n t.string :name\nend\n",
        ),
        (
            "result = create_table(:widgets).tap { work }\n",
            "result = create_table(:widgets, charset: \"test_value\").tap { work }\n",
        ),
        (
            "create_table(:widgets, {id: false})\n",
            "create_table(:widgets, {id: false, charset: \"test_value\"})\n",
        ),
        (
            "create_table(:widgets, {})\n",
            "create_table(:widgets, {charset: \"test_value\"})\n",
        ),
        (
            "create_table(:widgets, # widget comment\n)\n",
            "create_table(:widgets, charset: \"test_value\", # widget comment\n)\n",
        ),
    ] {
        assert_eq!(corrected(source, "charset"), expected);
    }
}

#[test]
fn dynamic_option_bundles_and_splats_remain_manual_findings() {
    for source in [
        "create_table :widgets, **options\n",
        "create_table :widgets, {**options}\n",
        "create_table :widgets, options\n",
        "create_table :widgets, \"charset\" => \"ascii\"\n",
        "create_table(*args)\n",
        "create_table <<~TABLE\n widgets\nTABLE\n",
    ] {
        let f = inspect(source, "charset");
        assert!(f.edit.is_none() && f.additional.is_empty(), "{source}");
    }
}

#[test]
fn both_options_can_be_added_without_erasing_each_other() {
    let first = corrected("create_table(:widgets)\n", "charset");
    assert_eq!(
        corrected(&first, "collation"),
        "create_table(:widgets, charset: \"test_value\", collation: \"test_value\")\n"
    );
}

#[test]
fn present_symbol_options_do_not_need_explicit_value_nodes() {
    for source in [
        "charset = \"ascii\"; collation = \"ascii_bin\"\ncreate_table(:widgets, charset:, collation:)\n",
        "charset = \"ascii\"; collation = \"ascii_bin\"\ncreate_table(:widgets, {charset:, collation:})\n",
        "create_table(:widgets, charset: nil, collation: nil)\n",
    ] {
        for key in ["charset", "collation"] {
            assert!(findings(source, key).is_empty(), "{key}: {source}");
        }
    }
}

#[test]
fn nested_options_do_not_count_as_table_options() {
    let f = inspect(
        "create_table(:widgets, metadata: {charset: \"ascii\"})\n",
        "charset",
    );
    assert!(f.edit.is_some());
}

#[test]
fn escaped_symbol_options_are_recognized_without_overriding_them() {
    for (key, source) in [
        (
            "charset",
            r#"create_table :widgets, :"char\x73et" => "ascii""#,
        ),
        (
            "collation",
            r#"create_table :widgets, :"colla\x74ion" => "ascii_bin""#,
        ),
    ] {
        assert!(findings(source, key).is_empty(), "{source}");
    }
}

#[test]
fn static_symbol_option_keys_allow_safe_insertion() {
    for source in [
        r#"create_table :widgets, %s|id| => :bigint"#,
        r#"create_table :widgets, :"\x69\x64" => :bigint"#,
        r#"create_table :widgets, :"id'" => :bigint"#,
        r#"create_table :widgets, :"id" => false"#,
    ] {
        assert_eq!(
            corrected(source, "charset"),
            format!("{source}, charset: \"test_value\"")
        );
    }
}

#[test]
fn multiline_comments_keep_options_inside_the_call() {
    let source = "create_table(\n# parity audit\n:widgets)\n";
    assert_eq!(
        corrected(source, "charset"),
        "create_table(\n# parity audit\n:widgets, charset: \"test_value\")\n"
    );
}

#[test]
fn ordinary_quoted_option_names_remain_recognized() {
    let source = r#"create_table :widgets, :"charset" => "ascii", :"collation" => "ascii_bin""#;
    for key in ["charset", "collation"] {
        assert!(findings(source, key).is_empty(), "{key}: {source}");
    }
}
