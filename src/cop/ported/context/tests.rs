use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

mod feasibility;

#[test]
fn enabled_correction_renders_the_replacement() {
    let (source, config) = fixture("foo");
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let n = tree.root_node();
    let render = Context::new(&source, &config);
    let f = Finding::new(n, "message").replace_with(&render, n, || "bar");
    assert_eq!(f.edit.unwrap().2, "bar");
}

#[test]
fn disabled_autocorrection_does_not_render_payloads() {
    let source = SourceFile::from_bytes("example.rb", b"foo".to_vec());
    let config = CopConfig {
        options: [(
            "AutoCorrect".into(),
            serde_yml::Value::String("disabled".into()),
        )]
        .into(),
        ..CopConfig::default()
    };
    let c = Context::new(&source, &config);
    assert!(c.config.autocorrect_disabled());
    let f = Finding::range(0, 3, "message")
        .edit_with(&c, 0, 3, || -> String { panic!("rendered disabled edit") });
    assert!(f.correctable);
    assert!(f.edit.is_none());
    assert_eq!(
        c.render_edit(|| panic!("rendered disabled shared payload")),
        ""
    );
}

#[test]
fn local_membership_is_cached_once_and_matches_full_model_across_scopes() {
    let (source, config) = fixture("def m(arg); bare; arg = 1; items.each { |arg| arg }; arg; end");
    assert_membership(&source, &config);
    let initialized = AtomicUsize::new(0);
    source.cached(0, || {
        initialized.fetch_add(1, Ordering::SeqCst);
        crate::model::LocalIdentifiers::default()
    });
    assert_eq!(initialized.load(Ordering::SeqCst), 0);
}

fn assert_membership(source: &SourceFile, config: &CopConfig) {
    let c = Context::new(source, config);
    let tree = crate::parse::parse_ruby(source).unwrap();
    let model = crate::model::build(source.as_bytes(), tree.clone());
    let expected = crate::model::LocalIdentifiers::from_scopes(&model.scopes);
    for n in descendants(tree.root_node()).filter(|n| n.kind() == "identifier") {
        assert_eq!(
            local_identifier(&c, n),
            expected.contains(n.start_byte()),
            "{}",
            c.text(n)
        );
    }
}

fn fixture(code: &str) -> (SourceFile, CopConfig) {
    (
        SourceFile::from_bytes("example.rb", code.as_bytes().to_vec()),
        CopConfig::default(),
    )
}

#[test]
fn streaming_walk_has_the_same_preorder_and_named_node_boundaries() {
    let source = SourceFile::from_bytes("example.rb", b"foo(1, bar(2)) # comment\nbaz\n".to_vec());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    fn collect(n: Node<'_>, result: &mut Vec<usize>) {
        result.push(n.id());
        let mut cursor = n.walk();
        for child in n.named_children(&mut cursor) {
            collect(child, result);
        }
    }
    for root in descendants(tree.root_node()) {
        let mut expected = vec![];
        collect(root, &mut expected);
        assert_eq!(
            descendants(root).map(|n| n.id()).collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn seeded_scope_storage_is_shared_and_identifier_index_stays_lazy() {
    let (source, config) = fixture("def m(arg); arg; end");
    let scopes = seed_scopes(&source);
    assert!(!scopes.released());
    let calls = AtomicUsize::new(0);
    source.cached(0, || {
        calls.fetch_add(1, Ordering::SeqCst);
        scopes.index()
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(scopes.released());
    assert_membership(&source, &config);
}

#[test]
fn streaming_walk_keeps_an_unnamed_root_without_leaving_its_subtree() {
    let (source, _) = fixture("a + b");
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let binary = tree.root_node().named_child(0).unwrap();
    let operator = binary.child_by_field_name("operator").unwrap();
    assert!(!operator.is_named());
    assert_eq!(descendants(operator).collect::<Vec<_>>(), [operator]);
}

fn seed_scopes(source: &SourceFile) -> std::sync::Arc<crate::model::LocalIdentifierScopes> {
    let model = crate::model::build(source.as_bytes(), crate::parse::parse_ruby(source).unwrap());
    let scopes = source.cached(0, || {
        crate::model::LocalIdentifierScopes::new(model.scopes.clone())
    });
    assert!(scopes.shares(&model.scopes));
    scopes
}

#[test]
fn numbered_parameters_are_local_only_inside_their_implicit_block() {
    let (source, config) = fixture(
        "items.each { _1; _2 }; _1; -> { _1 }; items.each { obj._1; _1() }; items.each { items.each { _1 }; outside }",
    );
    let context = Context::new(&source, &config);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let actual: Vec<_> = descendants(tree.root_node())
        .filter(|node| node.kind() == "identifier" && context.text(*node).starts_with('_'))
        .map(|node| {
            (
                context.text(node).to_string(),
                local_identifier(&context, node),
            )
        })
        .collect();
    assert_eq!(
        actual,
        vec![
            ("_1".into(), true),
            ("_2".into(), true),
            ("_1".into(), false),
            ("_1".into(), true),
            ("_1".into(), false),
            ("_1".into(), false),
            ("_1".into(), true)
        ]
    );
}

#[test]
fn implicit_numbered_parameters_include_outer_evaluated_definition_headers() {
    for definition in ["def _2.foo; end", "class C < _2; end", "class << _2; end"] {
        let (source, config) = fixture(&format!("items.inject({{}}) {{ {definition}; _1 }}"));
        let context = Context::new(&source, &config);
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let numbers: Vec<_> = descendants(tree.root_node())
            .filter(|n| n.kind() == "identifier" && matches!(context.text(*n), "_1" | "_2"))
            .collect();
        assert_eq!(numbers.len(), 2, "{definition}");
        assert!(
            numbers.into_iter().all(|n| local_identifier(&context, n)),
            "{definition}"
        );
    }
}

#[test]
fn reflective_reads_do_not_turn_binding_method_names_into_local_tokens() {
    let (source, config) =
        fixture("def m(arg, binding); binding; binding(); arg; end; def n(arg); binding; end");
    let context = Context::new(&source, &config);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let actual: Vec<_> = descendants(tree.root_node())
        .filter(|n| n.kind() == "identifier" && context.text(*n) == "binding")
        .map(|n| local_identifier(&context, n))
        .collect();
    assert_eq!(actual, vec![true, true, false, false]);
}
