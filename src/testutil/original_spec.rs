//! Differential replays captured from unchanged original Ruby cop specifications.

use crate::cop::walker::BatchedWalker;
use crate::cop::{Cop, CopConfig};
use crate::correction::CorrectionSet;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub fn assert_original_spec_parity(cop: &dyn Cop, fixture: &str) {
    let cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
    let mut failures = Vec::new();
    for case in cases.as_array().unwrap() {
        let original = case["original"].as_str().unwrap();
        let source = SourceFile::from_bytes(
            case.get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("test.rb"),
            original.as_bytes().to_vec(),
        );
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let map = CodeMap::from_tree(tree.root_node(), source.as_bytes());
        let config = CopConfig {
            options: case["config"]
                .as_object()
                .map(|object| {
                    object
                        .iter()
                        .map(|(key, value)| (key.clone(), serde_yml::to_value(value).unwrap()))
                        .collect()
                })
                .unwrap_or_default(),
            ..CopConfig::default()
        };
        let mut diagnostics = Vec::new();
        let mut corrections = Vec::new();
        cop.check_lines(&source, &config, &mut diagnostics, Some(&mut corrections));
        if cop.needs_file_model() {
            cop.check_file_model(
                &source,
                &crate::model::build(source.as_bytes(), tree.clone()),
                &config,
                &mut diagnostics,
                Some(&mut corrections),
            );
        } else {
            cop.check_source(
                &source,
                &tree,
                &map,
                &config,
                &mut diagnostics,
                Some(&mut corrections),
            );
        }
        BatchedWalker::new(vec![cop], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut corrections),
        );
        diagnostics
            .sort_by_key(|diagnostic| (diagnostic.location.line, diagnostic.location.column));
        let actual: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| {
                serde_json::json!({
                    "line": diagnostic.location.line, "column": diagnostic.location.column + 1,
                    "message": diagnostic.message, "length": diagnostic.highlight_length
                })
            })
            .collect();
        let expected: Vec<_> = case["ruby_diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|diagnostic| {
                serde_json::json!({
                    "line": diagnostic["line"], "column": diagnostic["column"],
                    "message": diagnostic["message"], "length": diagnostic["length"]
                })
            })
            .collect();
        let corrected = CorrectionSet::from_vec(corrections).apply(original.as_bytes());
        let expected_bytes = case
            .get("ruby_corrected")
            .unwrap_or(&case["ruby_safe"])
            .as_str()
            .unwrap()
            .as_bytes();
        if actual != expected || corrected != expected_bytes {
            failures.push(format!("{}\nsource: {:?}\nactual: {:?}\nexpected: {:?}\ncorrected: {:?}\nexpected correction: {:?}", case["id"], original, actual, expected, String::from_utf8_lossy(&corrected), String::from_utf8_lossy(expected_bytes)));
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} cases failed for {}:\n{}",
        failures.len(),
        cases.as_array().unwrap().len(),
        cop.name(),
        failures.join("\n\n")
    );
}
