use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn correction_feasibility_does_not_render_edit_text_in_lint_mode() {
    let (source, config) = fixture("foo");
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let n = tree.root_node();
    let lint = Context::with_edits(&source, &config, false);
    let f = Finding::new(n, "message")
        .replace_with(&lint, n, || -> String { panic!("rendered in lint mode") })
        .also_edit_with(&lint, 0, 0, || -> String { panic!("rendered extra edit") });
    assert!(f.correctable);
    assert!(f.edit.is_none() && f.additional.is_empty());
    assert_correctable_diagnostic(&source, &f);
}

fn assert_correctable_diagnostic(source: &SourceFile, f: &Finding) {
    let rule = super::super::Rule {
        name: "Style/Test",
        kinds: &[],
        check: |_, _| vec![],
        correctable: true,
        safe: true,
        extension: false,
    };
    let d = super::super::finding_diagnostic(&rule, source, f);
    assert!(d.correctable);
    assert!(!d.corrected);
}

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
    let expected = crate::model::LocalIdentifiers::from_model(&model);
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
