use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{argument_nodes, call_receiver, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct BeEmpty;
impl Cop for BeEmpty {
    fn name(&self) -> &'static str {
        "RSpec/BeEmpty"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !runner(source, node) {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        if !bare(source, receiver, b"expect") || argument_nodes(receiver).len() != 1 {
            return;
        }
        let args = argument_nodes(node);
        if args.is_empty() || args.len() > 2 {
            return;
        }
        let matcher = args[0];
        let values = argument_nodes(matcher);
        let empty = bare(source, matcher, b"contain_exactly") && values.is_empty()
            || bare(source, matcher, b"match_array")
                && values.len() == 1
                && values[0].kind() == "array"
                && values[0].named_child_count() == 0;
        if !empty {
            return;
        }
        let index = report(
            self,
            source,
            matcher.start_byte(),
            header_end(matcher),
            "Use `be_empty` matchers for checking an empty array.".into(),
            diagnostics,
        );
        if push_replace(
            &mut corrections,
            matcher.start_byte(),
            header_end(matcher),
            "be_empty",
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
