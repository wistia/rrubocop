use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct NoExpectationExample;
impl Cop for NoExpectationExample {
    fn name(&self) -> &'static str {
        "RSpec/NoExpectationExample"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) = name(source, node) else {
            return;
        };
        if call_receiver(node).is_some()
            || ![
                b"it".as_slice(),
                b"specify",
                b"example",
                b"scenario",
                b"its",
                b"fit",
                b"fspecify",
                b"fexample",
                b"fscenario",
                b"focus",
            ]
            .contains(&method)
        {
            return;
        }
        let Some(block) = call_block(node) else {
            return;
        };
        if skipped_metadata(source, node) {
            return;
        }
        let allowed = compiled_patterns(config, "AllowedPatterns");
        let mut pending = vec![block];
        while let Some(current) = pending.pop() {
            if !(current.kind() == "identifier" && local_binding(source, current))
                && call_receiver(current).is_none()
                && name(source, current).is_some_and(|name| {
                    expectations().contains(&name)
                        || [b"skip".as_slice(), b"pending"].contains(&name)
                        || allowed
                            .iter()
                            .any(|pattern| pattern.is_match(&String::from_utf8_lossy(name)))
                })
            {
                return;
            }
            let mut cursor = current.walk();
            pending.extend(current.named_children(&mut cursor));
        }
        report(
            self,
            source,
            node.start_byte(),
            node.end_byte(),
            "No expectation found in this example.".into(),
            diagnostics,
        );
    }
}
