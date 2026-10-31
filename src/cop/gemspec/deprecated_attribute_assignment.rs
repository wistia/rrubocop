//! Gemspec/DeprecatedAttributeAssignment — remove obsolete gemspec attrs.

use tree_sitter::{Node, Tree};

use crate::cop::shared::{call_method_name, for_each_descendant};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct DeprecatedAttributeAssignment;

const DEPRECATED: &[&[u8]] = &[
    b"test_files",
    b"date",
    b"specification_version",
    b"rubygems_version",
];

impl Cop for DeprecatedAttributeAssignment {
    fn name(&self) -> &'static str {
        "Gemspec/DeprecatedAttributeAssignment"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        tree: &Tree,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        for_each_descendant(tree.root_node(), |node| {
            if let Some(diag) = offense(self, source, node, &mut corrections) {
                diagnostics.push(diag);
            }
        });
    }
}

fn offense(
    cop: &DeprecatedAttributeAssignment,
    source: &SourceFile,
    node: Node<'_>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
) -> Option<Diagnostic> {
    let (method, left_start) = deprecated_lhs(source, node)?;
    let (line, column) = source.offset_to_line_col(left_start);
    let attr = std::str::from_utf8(method).unwrap_or("?");
    let mut diag = cop.diagnostic(
        source,
        line,
        column,
        format!("`{attr}` assignment is deprecated and will be removed from Rubygems."),
    );
    if let Some(corr) = corrections.as_mut() {
        let (start, end) = full_line_range(source.as_bytes(), node);
        corr.push(crate::correction::Correction {
            start,
            end,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    Some(diag)
}

fn deprecated_lhs<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<(&'a [u8], usize)> {
    if !matches!(node.kind(), "assignment" | "operator_assignment") {
        return None;
    }
    let left = node.child_by_field_name("left")?;
    if !matches!(left.kind(), "call" | "command_call") {
        return None;
    }
    let method = call_method_name(source, left)?;
    DEPRECATED
        .contains(&method)
        .then_some((method, left.start_byte()))
}

fn full_line_range(bytes: &[u8], node: Node<'_>) -> (usize, usize) {
    let mut start = node.start_byte();
    while start > 0 && bytes[start - 1] != b'\n' {
        start -= 1;
    }
    let mut end = node.end_byte();
    while end < bytes.len() && bytes[end] != b'\n' {
        end += 1;
    }
    (start, end + usize::from(end < bytes.len()))
}
