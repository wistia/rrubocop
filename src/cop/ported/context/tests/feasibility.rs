use super::*;

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
    let rule = super::super::super::Rule {
        name: "Style/Test",
        kinds: &[],
        check: |_, _| vec![],
        correctable: true,
        safe: true,
        extension: false,
    };
    let d = super::super::super::finding_diagnostic(&rule, source, f, &CopConfig::default());
    assert!(d.correctable);
    assert!(!d.corrected);
}
