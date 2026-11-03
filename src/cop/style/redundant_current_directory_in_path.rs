//! Style/RedundantCurrentDirectoryInPath.

use tree_sitter::Node;

use crate::cop::shared::{
    call_method_name, node_bytes, push_replace, semantic_argument_nodes as argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantCurrentDirectoryInPath;

fn literal_prefix_offset(source: &SourceFile, string: Node<'_>) -> Option<usize> {
    if string.kind() != "string" {
        return None;
    }
    let mut interpolated = false;
    crate::cop::shared::for_each_descendant(string, |node| {
        interpolated |= node.kind() == "interpolation";
    });
    if interpolated {
        return None;
    }
    let bytes = node_bytes(source, string);
    let start = match bytes {
        [b'\'' | b'"', ..] => 1,
        [b'%', b'q' | b'Q', _, ..] => 3,
        [b'%', _, ..] => 2,
        _ => return None,
    };
    bytes
        .get(start..)?
        .starts_with(b"./")
        .then_some(string.start_byte() + start)
}

fn content_offset(source: &SourceFile, argument: Node<'_>) -> Option<(usize, bool)> {
    if let Some(offset) = literal_prefix_offset(source, argument) {
        return Some((offset, true));
    }
    let value = crate::cop::ported::static_ruby_string(source, argument)?;
    let expected = value.strip_prefix("./")?;
    let bytes = node_bytes(source, argument);
    let offset = bytes.windows(2).position(|bytes| bytes == b"./")?;
    let mut rewritten = bytes[..offset].to_vec();
    rewritten.extend_from_slice(&bytes[offset + 2..]);
    let rewritten = SourceFile::from_bytes("path.rb", rewritten);
    let safe = crate::parse::parse_ruby(&rewritten)
        .ok()
        .and_then(|tree| {
            let literal = tree.root_node().named_child(0)?;
            crate::cop::ported::static_ruby_string(&rewritten, literal)
        })
        .is_some_and(|actual| actual == expected);
    Some((argument.start_byte() + offset, safe))
}

impl Cop for RedundantCurrentDirectoryInPath {
    fn name(&self) -> &'static str {
        "Style/RedundantCurrentDirectoryInPath"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"require_relative"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"require_relative")
            || node
                .child_by_field_name("operator")
                .is_some_and(|operator| node_bytes(source, operator) == b"&.")
        {
            return;
        }
        let Some(argument) = argument_nodes(node)
            .into_iter()
            .find(|argument| argument.kind() != "comment")
        else {
            return;
        };
        let Some((start, correctable)) = content_offset(source, argument) else {
            return;
        };
        let (line, column) = source.offset_to_line_col(start);
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Remove the redundant current directory path.".into(),
        );
        diagnostic.highlight_length = 2;
        diagnostic.correctable = correctable;
        if correctable {
            diagnostic.corrected =
                push_replace(&mut corrections, start, start + 2, "", self.name());
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_remove_a_different_encoded_path_segment() {
        let diagnostics = crate::testutil::run_cop_full(
            &RedundantCurrentDirectoryInPath,
            br#"require_relative "\x2e/foo/./bar""#,
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }

    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &RedundantCurrentDirectoryInPath,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/redundant_current_directory_in_path/parity.json"
            )),
        );
    }
}
