//! Source-level autocorrect edits (adapted from nitrocop).

use crate::diagnostic::Diagnostic;
mod credits;
mod ownership;
mod selection;
pub(crate) use ownership::{assign_owners, filter_disabled};

#[derive(Debug, Clone)]
pub struct Correction {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
    pub cop_name: &'static str,
    pub cop_index: usize,
    /// Owning offense start byte, independent of the edited ranges.
    pub owner: Option<usize>,
}

pub struct CorrectionSet {
    corrections: Vec<Correction>,
    /// Other offense owners whose identical edits were deduplicated.
    aliases: Vec<Vec<Correction>>,
}

impl CorrectionSet {
    pub fn from_vec(raw: Vec<Correction>) -> Self {
        let (corrections, aliases) = selection::select(raw)
            .into_iter()
            .map(|accepted| (accepted.edit, accepted.aliases))
            .unzip();
        Self {
            corrections,
            aliases,
        }
    }

    pub fn accepted(&self) -> &[Correction] {
        &self.corrections
    }

    pub fn into_accepted(self) -> Vec<Correction> {
        self.corrections
    }

    pub fn apply(&self, source: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(source.len());
        let mut cursor = 0;
        for c in &self.corrections {
            if c.start > cursor {
                result.extend_from_slice(&source[cursor..c.start]);
            }
            result.extend_from_slice(c.replacement.as_bytes());
            cursor = c.end;
        }
        if cursor < source.len() {
            result.extend_from_slice(&source[cursor..]);
        }
        result
    }

    pub fn is_empty(&self) -> bool {
        self.corrections.is_empty()
    }

    pub fn len(&self) -> usize {
        self.corrections.len()
    }
}

/// Clear `corrected` on diagnostics whose cop edit was dropped or not applied.
pub fn reconcile_corrected(
    diagnostics: &mut [Diagnostic],
    source: &crate::parse::source::SourceFile,
    set: &CorrectionSet,
) {
    if set.is_empty() {
        for d in diagnostics.iter_mut().filter(|d| d.corrected) {
            d.corrected = false;
        }
        return;
    }
    let credits = credits::Credits::new(set);
    for d in diagnostics.iter_mut().filter(|d| d.corrected) {
        let Some(off) = source.line_col_to_offset(d.location.line, d.location.column) else {
            d.corrected = false;
            continue;
        };
        if !credits.contains(&d.cop_name, off) {
            d.corrected = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::diagnostic::{Diagnostic, Location, Severity};
    use crate::parse::source::SourceFile;

    pub(super) fn edit(
        start: usize,
        end: usize,
        replacement: &str,
        cop: &'static str,
    ) -> Correction {
        Correction {
            start,
            end,
            replacement: replacement.into(),
            cop_name: cop,
            cop_index: 0,
            owner: None,
        }
    }

    pub(super) fn diag(cop: &str, column: usize) -> Diagnostic {
        Diagnostic {
            path: "a.rb".into(),
            cop_name: cop.into(),
            message: "m".into(),
            location: Location { line: 1, column },
            severity: Severity::Convention,
            corrected: true,
            correctable: true,
            source_line: String::new(),
            highlight_length: 1,
        }
    }

    #[test]
    fn identical_edits_keep_both_cops_corrected() {
        let source = SourceFile::from_bytes(Path::new("a.rb"), b"def foo(&block)\nend\n".to_vec());
        let set = CorrectionSet::from_vec(vec![
            edit(8, 14, "&", "Naming/BlockForwarding"),
            edit(8, 14, "&", "Style/ArgumentsForwarding"),
        ]);
        assert_eq!(set.len(), 1);
        assert_eq!(set.apply(source.as_bytes()), b"def foo(&)\nend\n");
        let mut diags = vec![
            diag("Naming/BlockForwarding", 8),
            diag("Style/ArgumentsForwarding", 8),
        ];
        reconcile_corrected(&mut diags, &source, &set);
        assert!(diags.iter().all(|d| d.corrected), "{diags:?}");
    }

    #[test]
    fn conflicting_overlap_does_not_credit_the_dropped_cop() {
        let source = SourceFile::from_bytes(Path::new("a.rb"), b"foo  \n".to_vec());
        let set = CorrectionSet::from_vec(vec![
            edit(3, 5, "", "Layout/Other"),
            edit(3, 5, " ", "Layout/TrailingWhitespace"),
        ]);
        let mut diags = vec![diag("Layout/TrailingWhitespace", 4)];
        reconcile_corrected(&mut diags, &source, &set);
        assert!(!diags[0].corrected);
    }
}

#[cfg(test)]
mod ownership_tests;
