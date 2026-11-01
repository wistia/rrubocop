use super::*;
use crate::{cop::CopConfig, parse::source::SourceFile};

fn handlers(root: Node<'_>) -> Vec<Node<'_>> {
    descendants(root)
        .filter_map(|n| match n.kind() {
            "rescue" => body(n),
            "rescue_modifier" => field(n, "handler"),
            _ => None,
        })
        .collect()
}

#[test]
fn targeted_summaries_match_scans_for_nested_and_disjoint_handlers() {
    for text in [
        "begin; work; rescue; begin; work; rescue; noop; end; end",
        "begin; work; rescue; begin; raise; rescue; noop; end; end",
        "begin; work; rescue; -> { Sentry.capture_exception($!) }; end; work rescue noop",
        "begin; work; rescue; raise=1; raise; end; work rescue Sentry&.capture_exception($!)",
        "begin; work; rescue; Rails.error.report($!); end; begin; work; rescue; noop; end",
    ] {
        compare(text);
    }
}

fn compare(text: &str) {
    let source = SourceFile::from_bytes("example.rb", text.as_bytes().to_vec());
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let summary = Reporters::new(&c, tree.root_node());
    for handler in handlers(tree.root_node()) {
        assert_eq!(
            summary.contains(&c, handler),
            descendants(handler).any(|n| reports(&c, n)),
            "{text}"
        );
    }
}

#[test]
fn no_file_scan_is_needed_before_a_handler_query() {
    let source = SourceFile::from_bytes("example.rb", b"def unrelated; Sentry.capture_exception($!); end; begin; work; rescue; noop; end; begin; work; rescue; noop; end".to_vec());
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let summary = Reporters::new(&c, tree.root_node());
    assert!(summary.0.as_ref().unwrap().lock().unwrap().is_empty());
    assert!(!summary.contains(&c, handlers(tree.root_node())[0]));
    assert_eq!(summary.0.unwrap().into_inner().unwrap().len(), 1);
}

#[test]
fn nested_queries_reuse_only_handler_summaries() {
    let text = "begin; work; rescue; begin; work; rescue; begin; work; rescue; noop; end; end; end";
    let source = SourceFile::from_bytes("example.rb", text.as_bytes().to_vec());
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let summary = Reporters::new(&c, tree.root_node());
    for handler in handlers(tree.root_node()) {
        assert!(!summary.contains(&c, handler));
        assert_eq!(summary.0.as_ref().unwrap().lock().unwrap().len(), 3);
    }
}
