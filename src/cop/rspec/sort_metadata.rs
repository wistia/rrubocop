use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

use crate::cop::shared::node_text;

pub struct SortMetadata;
impl Cop for SortMetadata {
    fn name(&self) -> &'static str {
        "RSpec/SortMetadata"
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
        if !metadata_call(source, node) {
            return;
        }
        let args = call_args(node);
        if args.len() < 2 {
            return;
        }
        let mut metadata_args: Vec<_> = args.iter().skip(1).copied().collect();
        if metadata_args
            .last()
            .is_some_and(|last| !symbol_node(*last) && !matches!(last.kind(), "pair" | "hash"))
        {
            metadata_args.pop();
        }
        let symbols: Vec<_> = metadata_args
            .iter()
            .filter(|arg| !matches!(arg.kind(), "pair" | "hash"))
            .copied()
            .collect();
        let pairs: Vec<_> = metadata_args
            .iter()
            .flat_map(|arg| {
                if arg.kind() == "hash" {
                    code_children(*arg)
                } else {
                    vec![*arg]
                }
            })
            .filter(|arg| arg.kind() == "pair")
            .collect();
        let mut sorted_symbols = symbols.clone();
        sorted_symbols.sort_by_cached_key(|symbol| {
            if symbol_node(*symbol) || symbol.kind() == "string" {
                crate::cop::ported::static_literal(source, *symbol)
                    .unwrap_or_else(|| node_text(source, *symbol))
                    .to_lowercase()
            } else {
                node_text(source, *symbol).to_lowercase()
            }
        });
        let mut sorted_pairs = pairs.clone();
        sorted_pairs.sort_by_cached_key(|pair| {
            pair.child_by_field_name("key")
                .map(|key| node_text(source, key).to_lowercase())
                .unwrap_or_default()
        });
        if sorted_symbols == symbols && sorted_pairs == pairs {
            return;
        }
        let metadata: Vec<_> = symbols.iter().chain(&pairs).copied().collect();
        let Some(first) = metadata.first() else {
            return;
        };
        let last = metadata.last().unwrap();
        let index = report(
            self,
            source,
            first.start_byte(),
            last.end_byte(),
            "Sort metadata alphabetically.".into(),
            diagnostics,
        );
        if metadata.iter().any(|value| !pure_literal(source, *value)) {
            return;
        }
        if corrections.is_none() {
            return;
        }
        let replacement = sorted_symbols
            .iter()
            .chain(&sorted_pairs)
            .map(|node| node_text(source, *node))
            .collect::<Vec<_>>()
            .join(", ");
        if preserving_replace(
            source,
            node,
            &mut corrections,
            first.start_byte(),
            last.end_byte(),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
