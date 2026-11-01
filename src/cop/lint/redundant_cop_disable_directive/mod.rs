mod audit;
mod directives;
mod removal;

pub(crate) use directives::nth_cop_token;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

use audit::audit_redundant_disables;
use directives::{cop_names, disable_marker};
use removal::scan_duplicate_cops;

/// Lint/RedundantCopDisableDirective — disable comments that suppress no offenses.
pub struct RedundantCopDisableDirective;

impl Cop for RedundantCopDisableDirective {
    fn name(&self) -> &'static str {
        "Lint/RedundantCopDisableDirective"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        tree: &tree_sitter::Tree,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let directives = crate::parse::directives::for_source(source, tree);
        for &line_no in directives.comment_lines() {
            let Some(line) = directives.comment_line(source, line_no) else {
                continue;
            };
            if let Some((_, rest)) = disable_marker(&line) {
                scan_duplicate_cops(
                    self,
                    source,
                    line_no,
                    &cop_names(rest),
                    diagnostics,
                    &mut corrections,
                );
            }
        }
    }

    fn audit_after_cops(
        &self,
        source: &SourceFile,
        offenses: &[Diagnostic],
        active: &[(&dyn Cop, &CopConfig)],
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        audit_redundant_disables(self, source, offenses, active, diagnostics, corrections);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Location, Severity};
    use crate::parse::source::byte_index_to_column;

    crate::cop_fixture_tests!(
        RedundantCopDisableDirective,
        "cops/lint/redundant_cop_disable_directive"
    );

    struct NamedCop(&'static str);

    impl Cop for NamedCop {
        fn name(&self) -> &'static str {
            self.0
        }

        fn uses_source_phase(&self) -> bool {
            true
        }
    }

    fn active_cops(names: &[&'static str]) -> (Vec<NamedCop>, Vec<CopConfig>) {
        let cops: Vec<NamedCop> = names.iter().map(|n| NamedCop(n)).collect();
        let cfgs = vec![CopConfig::default(); names.len()];
        (cops, cfgs)
    }

    fn active_refs<'a>(
        cops: &'a [NamedCop],
        cfgs: &'a [CopConfig],
    ) -> Vec<(&'a dyn Cop, &'a CopConfig)> {
        cops.iter()
            .zip(cfgs.iter())
            .map(|(c, cfg)| (c as &dyn Cop, cfg))
            .collect()
    }

    fn offense(cop: &str, line: usize) -> Diagnostic {
        Diagnostic {
            path: "test.rb".into(),
            location: Location { line, column: 0 },
            severity: Severity::Convention,
            cop_name: cop.into(),
            message: String::new(),
            corrected: false,
            correctable: false,
            source_line: String::new(),
            highlight_length: 1,
        }
    }

    #[test]
    fn duplicate_cop_in_disable() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = 1 # rubocop:disable Style/StringLiterals, Style/StringLiterals\n".to_vec(),
        );
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let code_map =
            crate::parse::codemap::CodeMap::from_tree(tree.root_node(), source.as_bytes());
        let mut diags = Vec::new();
        RedundantCopDisableDirective.check_source(
            &source,
            &tree,
            &code_map,
            &CopConfig::default(),
            &mut diags,
            None,
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Style/StringLiterals"));
    }

    #[test]
    fn needed_disable_not_reported() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"puts 'x' # rubocop:disable Rails/Output\n".to_vec(),
        );
        let offenses = vec![offense("Rails/Output", 1)];
        let mut diags = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &offenses,
            &active_refs(&cops, &cfgs),
            &mut diags,
            None,
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn redundant_block_disable() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# rubocop:disable Rails/Output\nRails.logger.debug 'x'\n# rubocop:enable Rails/Output\n"
                .to_vec(),
        );
        let mut diags = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &active_refs(&cops, &cfgs),
            &mut diags,
            None,
        );
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn redundant_multi_cop_block_disable() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# rubocop:disable Rails/Output, Layout/LineLength\nx = 1\n# rubocop:enable Rails/Output, Layout/LineLength\n"
                .to_vec(),
        );
        let mut diags = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output", "Layout/LineLength"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &active_refs(&cops, &cfgs),
            &mut diags,
            None,
        );
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn partial_block_enable_audits_each_cop_once() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# rubocop:disable Rails/Output, Layout/LineLength\nx = 1\n# rubocop:enable Rails/Output\n# rubocop:enable Layout/LineLength\n"
                .to_vec(),
        );
        let mut diags = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output", "Layout/LineLength"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &active_refs(&cops, &cfgs),
            &mut diags,
            None,
        );
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().any(|d| d.message.contains("Rails/Output")));
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("Layout/LineLength"))
        );
    }

    #[test]
    fn enable_all_closes_every_open_block() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# rubocop:disable Rails/Output, Layout/LineLength\nx = 1\n# rubocop:enable all\n"
                .to_vec(),
        );
        let mut diags = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output", "Layout/LineLength"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &active_refs(&cops, &cfgs),
            &mut diags,
            None,
        );
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.location.line == 1));
    }

    #[test]
    fn redundant_disable_column_on_multibyte_line() {
        let line = "Rails.logger.debug { \"Начинаю обновление\" } # rubocop:disable Rails/Output";
        let source = SourceFile::from_bytes("test.rb", format!("{line}\n").into_bytes());
        let mut diags = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &active_refs(&cops, &cfgs),
            &mut diags,
            None,
        );
        assert_eq!(diags.len(), 1);
        let col = byte_index_to_column(line, line.find("# rubocop:disable").unwrap());
        assert_eq!(diags[0].location.column, col);
        assert_eq!(
            diags[0].highlight_length,
            "# rubocop:disable Rails/Output".len()
        );
    }

    #[test]
    fn autocorrect_removes_inline_disable() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"Rails.logger.debug 'x' # rubocop:disable Rails/Output\n".to_vec(),
        );
        let mut diags = Vec::new();
        let mut corrs = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &active_refs(&cops, &cfgs),
            &mut diags,
            Some(&mut corrs),
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        assert_eq!(corrs.len(), 1);
        let fixed = crate::correction::CorrectionSet::from_vec(corrs).apply(source.as_bytes());
        assert_eq!(fixed, b"Rails.logger.debug 'x'\n");
    }

    #[test]
    fn needed_disable_in_same_directive_not_reported() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = 1 # rubocop:disable Rails/Output, Layout/LineLength\n".to_vec(),
        );
        let offenses = vec![offense("Layout/LineLength", 1)];
        let mut diags = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output", "Layout/LineLength"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &offenses,
            &active_refs(&cops, &cfgs),
            &mut diags,
            None,
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Rails/Output"));
    }

    #[test]
    fn autocorrect_partial_multi_disable() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = 1 # rubocop:disable Rails/Output, Layout/LineLength\n".to_vec(),
        );
        let mut corrs = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output", "Layout/LineLength"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[offense("Layout/LineLength", 1)],
            &active_refs(&cops, &cfgs),
            &mut Vec::new(),
            Some(&mut corrs),
        );
        let fixed = crate::correction::CorrectionSet::from_vec(corrs).apply(source.as_bytes());
        assert_eq!(fixed, b"x = 1 # rubocop:disable Layout/LineLength\n");
    }

    #[test]
    fn autocorrect_removes_comment_only_disable_line() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# rubocop:disable Rails/Output\nx = 1\n".to_vec(),
        );
        let mut diags = Vec::new();
        let mut corrs = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &active_refs(&cops, &cfgs),
            &mut diags,
            Some(&mut corrs),
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let fixed = crate::correction::CorrectionSet::from_vec(corrs).apply(source.as_bytes());
        assert_eq!(fixed, b"x = 1\n");
    }

    #[test]
    fn autocorrect_removes_block_disable_leaves_enable() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# rubocop:disable Rails/Output\nRails.logger.debug 'x'\n# rubocop:enable Rails/Output\n"
                .to_vec(),
        );
        let mut corrs = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &active_refs(&cops, &cfgs),
            &mut Vec::new(),
            Some(&mut corrs),
        );
        let fixed = crate::correction::CorrectionSet::from_vec(corrs).apply(source.as_bytes());
        assert_eq!(
            fixed,
            b"Rails.logger.debug 'x'\n# rubocop:enable Rails/Output\n"
        );
    }

    #[test]
    fn autocorrect_keeps_block_disable_when_one_cop_still_needed() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# rubocop:disable Rails/Output, Layout/LineLength\nx = 1\n# rubocop:enable Rails/Output, Layout/LineLength\n"
                .to_vec(),
        );
        let mut corrs = Vec::new();
        let (cops, cfgs) = active_cops(&["Rails/Output", "Layout/LineLength"]);
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[offense("Layout/LineLength", 2)],
            &active_refs(&cops, &cfgs),
            &mut Vec::new(),
            Some(&mut corrs),
        );
        let fixed = crate::correction::CorrectionSet::from_vec(corrs).apply(source.as_bytes());
        assert_eq!(
            fixed,
            b"# rubocop:disable Layout/LineLength\nx = 1\n# rubocop:enable Rails/Output, Layout/LineLength\n"
        );
    }

    #[test]
    fn skip_disable_for_unrun_cop() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"User.update_all(active: false) # rubocop:disable Rails/SkipsModelValidations\n"
                .to_vec(),
        );
        let mut diags = Vec::new();
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &[],
            &mut diags,
            None,
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn bare_disable_is_not_treated_as_all() {
        let source = SourceFile::from_bytes("test.rb", b"scope: 'x', # rubocop:disable\n".to_vec());
        let (cops, cfgs) = active_cops(&["Style/StringLiterals"]);
        let mut diags = Vec::new();
        audit_redundant_disables(
            &RedundantCopDisableDirective,
            &source,
            &[],
            &active_refs(&cops, &cfgs),
            &mut diags,
            None,
        );
        assert!(diags.is_empty());
    }
}
