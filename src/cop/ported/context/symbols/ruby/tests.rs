use super::*;

#[test]
fn static_strings_match_pinned_ruby_ast_values() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ruby-static-string-values.json"
    )))
    .unwrap();
    for case in cases.as_array().unwrap() {
        verify(case);
    }
}

fn verify(case: &serde_json::Value) {
    let source = SourceFile::from_bytes(
        "value.rb",
        case["source"].as_str().unwrap().as_bytes().to_vec(),
    );
    assert_eq!(values(&source), case["expected"], "{}", case["source"]);
}

fn values(source: &SourceFile) -> serde_json::Value {
    let tree = crate::parse::parse_ruby(source).unwrap();
    let root = tree.root_node();
    assert!(!root.has_error());
    serde_json::to_value(decoded(
        &Context::new(source, &CopConfig::default()),
        args(root.named_child(0).unwrap()),
    ))
    .unwrap()
}

fn decoded(context: &Context<'_>, nodes: Vec<Node<'_>>) -> Vec<Option<String>> {
    nodes
        .into_iter()
        .map(|node| context.ruby_static_string(node).map(Cow::into_owned))
        .collect()
}
