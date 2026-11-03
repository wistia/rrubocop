use super::super::context::*;
use tree_sitter::Node;

pub(super) fn interpolation(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let text = c.text(n);
    if !(text.starts_with('\'') || text.starts_with("%q"))
        || c.static_symbol(n).is_some()
        || c.ruby_string_parts(n).is_none_or(|parts| parts.len() != 1)
        || ancestors(n).any(|parent| parent.kind() == "heredoc_body")
        || !has_interpolation(text)
    {
        return vec![];
    }
    let mut finding = Finding::new(
        n,
        "Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.",
    );
    finding.highlight_length = Some(text.replace("\r\n", "\n").chars().count());
    vec![finding]
}

fn has_interpolation(text: &str) -> bool {
    text.split('\n').any(|line| {
        line.match_indices("#{").any(|(offset, _)| {
            (offset == 0 || line.as_bytes()[offset - 1] != b'\\')
                && line[offset + 2..].contains('}')
        })
    })
}

#[cfg(test)]
mod tests {
    fn replay(fixture: &str) {
        let mut cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let registry = crate::cop::CopRegistry::default_registry();
        let cop = registry.get("Lint/InterpolationCheck").unwrap();
        for case in cases.as_array_mut().unwrap() {
            if let Some(diagnostics) = case.get("ruby_parser_diagnostics") {
                assert!(
                    !diagnostics
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|d| matches!(d["level"].as_str(), Some("error" | "fatal")))
                );
            }
            assert!(case.get("ruby_cop_error").is_none());
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
            let config = crate::cop::CopConfig {
                options: case["config"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(key, value)| (key.clone(), serde_yml::to_value(value).unwrap()))
                    .collect(),
                ..Default::default()
            };
            let mut diagnostics = Vec::new();
            let mut corrections = Vec::new();
            crate::cop::walker::BatchedWalker::new(vec![cop], vec![&config]).walk(
                &source,
                tree.root_node(),
                &mut diagnostics,
                Some(&mut corrections),
            );
            assert!(diagnostics.iter().all(|diagnostic| !diagnostic.correctable));
            assert!(corrections.is_empty());
            case["ruby_corrected"] = case["original"].clone();
        }
        crate::testutil::assert_original_spec_parity(cop, &serde_json::to_string(&cases).unwrap());
    }
    #[test]
    fn unchanged_original_examples_preserve_literal_expressions() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/interpolation_check/original-parity.json"
        )));
    }
    #[test]
    fn final_native_states_match_fresh_original_and_parse_cleanly() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/interpolation_check/final-states.json"
        )));
    }
    #[test]
    fn original_rewrites_activate_dynamic_string_nodes() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/interpolation_check/original-independent.json"
        )))
        .unwrap();
        for case in cases
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case.get("rust_corrected").is_some())
        {
            let count = |key: &str| {
                case[key]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|kind| **kind == "dstr")
                    .count()
            };
            assert!(
                count("ruby_corrected_ast_types") > count("ruby_ast_types"),
                "{}",
                case["id"]
            );
            assert_eq!(case["rust_corrected"], case["original"]);
            assert_eq!(case["rust_correctable"], false);
        }
    }
    #[test]
    fn independent_original_branches_preserve_literal_expressions() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/interpolation_check/original-independent.json"
        )));
    }
}
