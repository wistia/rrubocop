use super::super::context::*;
use tree_sitter::Node;

pub(super) fn empty_when(c: &Context<'_>, node: Node<'_>) -> Vec<Finding> {
    if has_body(node) {
        return vec![];
    }
    let allow_comments = !matches!(
        c.config.options.get("AllowComments"),
        Some(serde_yml::Value::Bool(false) | serde_yml::Value::Null)
    );
    if allow_comments && when_comments(node) {
        return vec![];
    }
    let mut cursor = node.walk();
    let end = node
        .children_by_field_name("pattern", &mut cursor)
        .last()
        .map_or(node.end_byte(), |pattern| pattern.end_byte());
    let mut finding = Finding::range(
        node.start_byte(),
        end,
        "Avoid `when` branches without a body.",
    );
    finding.highlight_length = Some(
        String::from_utf8_lossy(&c.source.as_bytes()[node.start_byte()..end])
            .replace("\r\n", "\n")
            .chars()
            .count(),
    );
    vec![finding]
}

fn has_body(node: Node<'_>) -> bool {
    field(node, "body").is_some_and(|body| {
        kids(body)
            .iter()
            .any(|node| node.kind() != "empty_statement")
    })
}

fn comment_end_line(node: Node<'_>) -> usize {
    let Some(parent) = node.parent() else {
        return node.end_position().row;
    };
    for sibling in kids(parent)
        .into_iter()
        .skip_while(|sibling| *sibling != node)
        .skip(1)
    {
        if sibling.kind() == "when" {
            return sibling.start_position().row;
        }
        if sibling.kind() == "else"
            && let Some(body) = kids(sibling)
                .into_iter()
                .find(|child| child.kind() != "empty_statement")
        {
            return body.start_position().row;
        }
    }
    let mut cursor = parent.walk();
    parent
        .children(&mut cursor)
        .find(|child| child.kind() == "end")
        .map_or(parent.end_position().row, |end| end.start_position().row)
}

fn when_comments(node: Node<'_>) -> bool {
    let (start, end) = (node.start_position().row, comment_end_line(node));
    descendants(ancestors(node).last().unwrap_or(node)).any(|child| {
        child.kind() == "comment"
            && child.start_position().row >= start
            && child.start_position().row < end
    })
}

#[cfg(test)]
mod tests {
    fn replay(fixture: &str) {
        let registry = crate::cop::CopRegistry::default_registry();
        let cop = registry.get("Lint/EmptyWhen").unwrap();
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
            "/tests/fixtures/cops/lint/empty_when/original-parity.json"
        )));
    }
    #[test]
    fn independent_comment_windows() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/empty_when/original-independent.json"
        )));
    }
}
