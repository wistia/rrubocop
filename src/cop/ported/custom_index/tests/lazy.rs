use super::*;

#[test]
fn each_lookup_builds_only_its_requested_component() {
    for component in ["methods", "policies", "queues"] {
        check_component(component);
    }
}
fn check_component(component: &str) {
    let source = SourceFile::from_bytes("example.rb", SOURCE.as_bytes().to_vec());
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let summary = index(&c, tree.root_node());
    let node = component_node(&c, tree.root_node(), component);
    match component {
        "methods" => {
            assert!(summary.method(&c, node, "nested", false));
        }
        "policies" => {
            assert!(summary.policy_scope(&c, node));
        }
        _ => {
            assert!(summary.worker_queue(&c, node).1);
        }
    }
    check_initialized(&summary, component);
}
fn check_initialized(summary: &Index, component: &str) {
    assert_eq!(summary.methods.get().is_some(), component == "methods");
    assert_eq!(summary.policies.get().is_some(), component == "policies");
    assert_eq!(summary.queues.get().is_some(), component == "queues");
}

fn component_node<'a>(c: &Context<'_>, root: Node<'a>, component: &str) -> Node<'a> {
    let (kind, prefix) = match component {
        "methods" => ("call", "attribute"),
        "policies" => ("class", "class OuterPolicy"),
        _ => ("class", "class Worker"),
    };
    descendants(root)
        .find(|n| n.kind() == kind && c.text(*n).starts_with(prefix))
        .unwrap()
}

#[test]
fn absent_worker_marker_does_not_initialize_any_structural_index() {
    let source = SourceFile::from_bytes(
        "example.rb",
        b"class NotAWorker; def perform; work; end; end".to_vec(),
    );
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let summary = index(&c, tree.root_node());
    assert!(!summary.may_include_worker(&c));
    assert!(summary.methods.get().is_none());
    assert!(summary.policies.get().is_none());
    assert!(summary.queues.get().is_none());
}
