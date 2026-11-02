use super::*;

#[test]
fn pinned_original_omitted_value_roles() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/model/underscore-shorthand-roles.json"
    )))
    .unwrap();
    for case in cases.as_array().unwrap() {
        assert!(
            case["parser_errors"].as_array().unwrap().is_empty(),
            "{}",
            case["id"]
        );
        let text = case["original"].as_str().unwrap();
        let source = SourceFile::from_bytes("probe.rb", text.as_bytes().to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let model = crate::model::build(source.as_bytes(), tree.clone());
        let config = CopConfig::default();
        let context = Context::new(&source, &config);
        for role in case["ruby_roles"].as_array().unwrap() {
            let byte = role["byte"].as_u64().unwrap() as usize;
            let expected = role["local"].as_bool().unwrap();
            assert_eq!(
                model.vcall_sites.contains(&byte),
                role["node_type"] == "send",
                "vcall {}: {role}",
                case["id"]
            );
            let key = tree
                .root_node()
                .named_descendant_for_byte_range(byte, byte + 1)
                .unwrap();
            assert_eq!(
                local_identifier(&context, key),
                expected,
                "{}: {role}",
                case["id"]
            );
        }
    }
}

#[test]
fn ignored_shorthand_keeps_usage_accounting_unchanged() {
    let text = b"_value=1; {_value:}; {_value:}\n";
    let source = SourceFile::from_bytes("probe.rb", text.to_vec());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let model = crate::model::build(source.as_bytes(), tree);
    assert_eq!(model.scopes[0].entries["_value"].reads.len(), 0);
    assert_eq!(model.scopes[0].entries["_value"].writes.len(), 1);
    assert!(model.vcall_sites.is_empty());
}
