use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct HookArgument;
impl Cop for HookArgument {
    fn name(&self) -> &'static str {
        "RSpec/HookArgument"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) = name(source, node) else {
            return;
        };
        if !hooks().contains(&method) || call_block(node).is_none() {
            return;
        }
        let args = call_args(node);
        if args.len() > 1 {
            return;
        }
        let scope = args.first().and_then(|arg| symbol(source, *arg));
        if !args.is_empty() && !matches!(scope, Some(b"each" | b"example")) {
            return;
        }
        let style = config.get_str("EnforcedStyle", "implicit");
        if scope == Some(style.as_bytes()) || scope.is_none() && style == "implicit" {
            return;
        }
        let target = selector(node);
        let end = if scope.is_some() {
            header_end(node)
        } else {
            target.end_byte()
        };
        let message = if style == "implicit" {
            format!(
                "Omit the default `:{}` argument for RSpec hooks.",
                String::from_utf8_lossy(scope.unwrap())
            )
        } else {
            format!("Use `:{style}` for RSpec hooks.")
        };
        let index = report(self, source, node.start_byte(), end, message, diagnostics);
        let replacement = if style == "implicit" {
            String::new()
        } else {
            format!("(:{style})")
        };
        if preserving_replace(
            source,
            node,
            &mut corrections,
            target.end_byte(),
            header_end(node),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
