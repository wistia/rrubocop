use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct MissingExampleGroupArgument;
impl Cop for MissingExampleGroupArgument {
    fn name(&self) -> &'static str {
        "RSpec/MissingExampleGroupArgument"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) = bare_rspec_call(source, node) else {
            return;
        };
        if ![
            b"describe".as_slice(),
            b"context",
            b"feature",
            b"example_group",
            b"fdescribe",
            b"fcontext",
            b"xdescribe",
            b"xcontext",
        ]
        .contains(&method)
            || call_block(node).is_none()
            || !call_args(node).is_empty()
        {
            return;
        }
        report(
            self,
            source,
            node.start_byte(),
            node.end_byte(),
            format!(
                "The first argument to `{}` should not be empty.",
                String::from_utf8_lossy(method)
            ),
            diagnostics,
        );
    }
}
