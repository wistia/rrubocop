//! Layout/LeadingCommentSpace.

use tree_sitter::Tree;

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct LeadingCommentSpace;

fn missing_space_after_hash(text: &[u8]) -> bool {
    if text.is_empty() || text[0] != b'#' {
        return false;
    }
    if text.starts_with(b"#++") || text.starts_with(b"#--") {
        return false;
    }
    let hash_run = text.iter().take_while(|&&b| b == b'#').count();
    match text.get(hash_run) {
        None => false,
        Some(b' ' | b'\t' | b'\r' | b'\n' | b'\x0b' | b'\x0c' | b'=') => false,
        Some(_) => true,
    }
}

fn skip_rbs_annotation(config: &CopConfig, text: &[u8]) -> bool {
    if !config.get_bool("AllowRBSInlineAnnotation", false) {
        return false;
    }
    text.starts_with(b"#:") || text.starts_with(b"#[") || text.starts_with(b"#|")
}

fn skip_steep_annotation(config: &CopConfig, text: &[u8]) -> bool {
    config.get_bool("AllowSteepAnnotation", false)
        && (text.starts_with(b"#:") || text.starts_with(b"#$"))
}

fn skip_doxygen(config: &CopConfig, text: &[u8]) -> bool {
    config.get_bool("AllowDoxygenCommentStyle", false) && text.starts_with(b"#*")
}

fn skip_gemfile_ruby(config: &CopConfig, source: &SourceFile, text: &[u8]) -> bool {
    config.get_bool("AllowGemfileRubyComment", false)
        && text.starts_with(b"#ruby")
        && std::path::Path::new(source.path_str())
            .file_name()
            .and_then(|n| n.to_str())
            == Some("Gemfile")
}

fn skip_config_ru(source: &SourceFile, text: &[u8], line: usize) -> bool {
    text.starts_with(b"#\\")
        && line == 1
        && std::path::Path::new(source.path_str())
            .file_name()
            .and_then(|n| n.to_str())
            == Some("config.ru")
}

fn skip_allowed_comment(source: &SourceFile, config: &CopConfig, text: &[u8], line: usize) -> bool {
    line == 1 && (text.starts_with(b"#!") || skip_config_ru(source, text, line))
        || skip_rbs_annotation(config, text)
        || skip_steep_annotation(config, text)
        || skip_doxygen(config, text)
        || skip_gemfile_ruby(config, source, text)
}

impl Cop for LeadingCommentSpace {
    fn name(&self) -> &'static str {
        "Layout/LeadingCommentSpace"
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
        tree: &Tree,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = code_map;
        let bytes = source.as_bytes();
        let initial_count = diagnostics.len();
        for comment in shared::collect_comments(tree.root_node()) {
            let start = comment.start_byte();
            let text = &bytes[start..comment.end_byte()];
            let (line, column) = source.offset_to_line_col(start);
            if !missing_space_after_hash(text) || skip_allowed_comment(source, config, text, line) {
                continue;
            }
            let mut diagnostic =
                self.diagnostic(source, line, column, "Missing space after `#`.".into());
            diagnostic.highlight_length = String::from_utf8_lossy(text)
                .trim_end_matches('\r')
                .chars()
                .count();
            diagnostic.corrected =
                shared::push_replace(&mut corrections, start + 1, start + 1, " ", self.name());
            diagnostics.push(diagnostic);
        }
        if diagnostics.len() > initial_count {
            for (offset, window) in bytes.windows(2).enumerate() {
                if window == b"\r\n" {
                    shared::push_replace(&mut corrections, offset, offset + 1, "", self.name());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::{assert_cop_no_offenses_full_with_config, assert_cop_offenses_full};

    crate::cop_fixture_tests!(LeadingCommentSpace, "cops/layout/leading_comment_space");

    #[test]
    fn independent_original_states() {
        crate::testutil::assert_original_spec_parity(
            &LeadingCommentSpace,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/leading_comment_space/independent-parity.json"
            )),
        );
    }
    #[test]
    fn original_ruby_states() {
        crate::testutil::assert_original_spec_parity(
            &LeadingCommentSpace,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/leading_comment_space/original-parity.json"
            )),
        );
    }
    #[test]
    fn rbs_inline_annotation_allowed() {
        let mut config = CopConfig::default();
        config.options.insert(
            "AllowRBSInlineAnnotation".into(),
            serde_yml::Value::Bool(true),
        );
        assert_cop_no_offenses_full_with_config(
            &LeadingCommentSpace,
            b"x = false #:page_load\n",
            config,
        );
    }

    #[test]
    fn rbs_inline_annotation_offense_when_disabled() {
        assert_cop_offenses_full(
            &LeadingCommentSpace,
            b"x = false #:page_load\n          ^ Layout/LeadingCommentSpace: Missing space after `#`.\n",
        );
    }
}
