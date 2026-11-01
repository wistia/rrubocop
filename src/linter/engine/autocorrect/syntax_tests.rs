use super::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;

struct InvalidRewrite;
impl Cop for InvalidRewrite {
    fn name(&self) -> &'static str {
        "Test/InvalidRewrite"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: tree_sitter::Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(source, line, column, "Propose invalid syntax".into());
        if let Some(corrections) = corrections {
            corrections.push(Correction {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement: "foo(".into(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diagnostic.corrected = true;
        }
        diagnostics.push(diagnostic);
    }
}
fn registry() -> CopRegistry {
    let mut registry = CopRegistry::new();
    registry.register(Box::new(InvalidRewrite));
    registry
}
#[test]
fn invalid_candidate_never_replaces_the_original_buffer() {
    let config = ResolvedConfig::empty();
    let registry = registry();
    let filters = CopFilterSet::build(&config, &registry);
    let mut bytes = b"foo()\n".to_vec();
    let error = lint_bytes_autocorrect(
        Path::new("probe.rb"),
        &mut bytes,
        &config,
        &registry,
        &filters,
        None,
        &[],
        AutocorrectMode::All,
        false,
    )
    .unwrap_err();
    assert!(error.to_string().contains("would introduce invalid Ruby"));
    assert_eq!(bytes, b"foo()\n");
}
#[test]
fn invalid_candidate_never_replaces_the_original_file() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("probe.rb");
    std::fs::write(&file, "foo()\n").unwrap();
    let config = ResolvedConfig::empty();
    let registry = registry();
    let filters = CopFilterSet::build(&config, &registry);
    let configs = PreparedConfigs::new(&config, &registry);
    let prepared = PreparedLint {
        config: &config,
        registry: &registry,
        filters: &filters,
        configs: &configs,
    };
    assert!(
        lint_file_autocorrect(
            &SourceFile::from_path(&file).unwrap(),
            &prepared,
            None,
            &[],
            AutocorrectMode::Safe,
            false
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(directory.path().join("probe.rb")).unwrap(),
        b"foo()\n"
    );
}
