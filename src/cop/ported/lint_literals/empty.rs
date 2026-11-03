use super::super::context::*;
use tree_sitter::Node;

pub(super) fn empty(c: &Context<'_>, node: Node<'_>) -> Vec<Finding> {
    if kids(node)
        .iter()
        .any(|child| child.kind() != "empty_statement")
    {
        return vec![];
    }
    let mut finding = Finding::new(node, "Avoid empty expressions.");
    finding.highlight_length = Some(c.text(node).replace("\r\n", "\n").chars().count());
    vec![finding]
}

#[cfg(test)]
mod tests {
    fn replay(fixture: &str) {
        let registry = crate::cop::CopRegistry::default_registry();
        let cop = registry.get("Lint/EmptyExpression").unwrap();
        let cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        for case in cases.as_array().unwrap() {
            let source = crate::parse::source::SourceFile::from_bytes(
                "probe.rb",
                case["original"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(
                !tree.root_node().has_error(),
                "{} {}",
                case["id"],
                tree.root_node().to_sexp()
            );
            assert_eq!(
                case["ruby_parser_diagnostics"],
                serde_json::json!([]),
                "{}",
                case["id"]
            );
            assert_eq!(case["ruby_corrected"], case["original"], "{}", case["id"]);
            let mut diagnostics = Vec::new();
            let mut corrections = Vec::new();
            let config = crate::cop::CopConfig::default();
            crate::cop::walker::BatchedWalker::new(vec![cop], vec![&config]).walk(
                &source,
                tree.root_node(),
                &mut diagnostics,
                Some(&mut corrections),
            );
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| !diagnostic.correctable && !diagnostic.corrected)
            );
            assert!(corrections.is_empty());
        }
        crate::testutil::assert_original_spec_parity(cop, fixture);
    }
    #[test]
    fn original_examples() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/empty_expression/original-parity.json"
        )));
    }
    #[test]
    fn independent_states() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/empty_expression/original-independent.json"
        )));
    }
}
