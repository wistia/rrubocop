use super::*;
use crate::cop::{Cop, CopConfig};

struct MapProbe(bool);
impl Cop for MapProbe {
    fn name(&self) -> &'static str {
        "Test/MapProbe"
    }
    fn uses_source_phase(&self) -> bool {
        true
    }
    fn needs_code_map(&self) -> bool {
        self.0
    }
    fn check_source(
        &self,
        _source: &SourceFile,
        _tree: &tree_sitter::Tree,
        map: &CodeMap,
        _config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        assert_eq!(map.covers(1), self.0);
    }
}
#[test]
fn plain_source_cops_only_build_literal_regions_when_requested() {
    let source = SourceFile::from_bytes("t.rb", b"'literal'\n".to_vec());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let config = CopConfig::default();
    for requested in [false, true] {
        let cop = MapProbe(requested);
        run_plain_source_cops(
            &source,
            &tree,
            &[(&cop, &config, 0)],
            AutocorrectMode::Off,
            &mut Vec::new(),
            &mut None,
        );
    }
}
#[test]
fn custom_regexp_reference_check_does_not_request_literal_regions() {
    let registry = crate::cop::CopRegistry::default_registry();
    assert!(
        !registry
            .get("Lint/OutOfRangeRegexpRef")
            .unwrap()
            .needs_code_map()
    );
}
