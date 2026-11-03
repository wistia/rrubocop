use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::{numbered_block, separate};
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct EmptyLineAfterSubject;
impl Cop for EmptyLineAfterSubject {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterSubject"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        let Some(method) = name(source, node) else {
            return;
        };
        if call_receiver(node).is_some()
            || ![b"subject".as_slice(), b"subject!"].contains(&method)
            || call_block(node).is_none()
            || numbered_block(source, node)
            || !inside_root_spec_group(source, node)
        {
            return;
        }
        separate(
            self,
            source,
            node,
            format!(
                "Add an empty line after `{}`.",
                String::from_utf8_lossy(method)
            ),
            diagnostics,
            &mut corrections,
        );
    }
}
