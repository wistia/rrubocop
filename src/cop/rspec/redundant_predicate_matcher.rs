use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct RedundantPredicateMatcher;
impl Cop for RedundantPredicateMatcher {
    fn name(&self) -> &'static str {
        "RSpec/RedundantPredicateMatcher"
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
        let _ = config;
        if safe_send(source, node) {
            return;
        }
        let Some(method) = name(source, node) else {
            return;
        };
        if ![
            b"be_all".as_slice(),
            b"be_cover",
            b"be_end_with",
            b"be_eql",
            b"be_equal",
            b"be_exist",
            b"be_exists",
            b"be_include",
            b"be_match",
            b"be_respond_to",
            b"be_start_with",
        ]
        .contains(&method)
        {
            return;
        }
        if crate::cop::rspec::helpers::call_block(node).is_some()
            && !crate::cop::rspec::separation::numbered_block(source, node)
        {
            return;
        }
        let args = call_args(node);
        if args.is_empty() {
            return;
        }
        if method == b"be_all"
            && (safe_send(source, args[0])
                || !(args[0].kind() == "call"
                    || args[0].kind() == "identifier" && !local_binding(source, args[0]))
                || crate::cop::rspec::helpers::call_block(args[0]).is_some())
        {
            return;
        }
        let current = String::from_utf8_lossy(method);
        let replacement = if method == b"be_exists" {
            "exist"
        } else {
            current.strip_prefix("be_").unwrap()
        };
        let index = report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            format!("Use `{replacement}` instead of `{current}`."),
            diagnostics,
        );
        diagnostics[index].correctable = method != b"be_all";
        if method != b"be_all" {
            let target = selector(node);
            if preserving_replace(
                source,
                node,
                &mut corrections,
                target.start_byte(),
                target.end_byte(),
                replacement,
                self.name(),
            ) {
                diagnostics[index].corrected = true;
            }
        }
    }
}
