use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct BeforeAfterAll;
impl Cop for BeforeAfterAll {
    fn name(&self) -> &'static str {
        "RSpec/BeforeAfterAll"
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
        if safe_send(source, node) {
            return;
        }
        let Some(method) = name(source, node) else {
            return;
        };
        if ![b"before".as_slice(), b"after"].contains(&method) {
            return;
        }
        let args = call_args(node);
        if args.len() != 1
            || !symbol_node(args[0])
            || !matches!(symbol(source, args[0]), Some(b"all" | b"context"))
        {
            return;
        }
        let end = header_end(node);
        let hook = String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end]);
        report(
            self,
            source,
            node.start_byte(),
            end,
            format!(
                "Beware of using `{hook}` as it may cause state to leak between tests. If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, then records created in `{hook}` are not automatically rolled back."
            ),
            diagnostics,
        );
    }
}
