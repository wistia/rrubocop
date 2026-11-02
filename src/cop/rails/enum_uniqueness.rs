//! Report every repeated enum value after the first, grouped in original value order.
use crate::cop::shared::ruby_expression::signature;
use crate::cop::shared::{argument_nodes, call_method_name, call_receiver, named_kids, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct EnumUniqueness;
fn collection(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "array" | "string_array" | "symbol_array" | "hash"
    )
}
impl EnumUniqueness {
    fn report(
        &self,
        source: &SourceFile,
        key: Node<'_>,
        values: Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let name = crate::cop::ported::static_literal(source, key)
            .unwrap_or_else(|| node_text(source, key));
        let elements: Vec<_> = named_kids(values)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .filter_map(|n| {
                if values.kind() == "hash" {
                    n.child_by_field_name("value")
                } else {
                    Some(n)
                }
            })
            .collect();
        let mut groups: Vec<(String, Vec<Node<'_>>)> = vec![];
        for item in elements {
            let id = signature(source, item);
            if let Some((_, items)) = groups.iter_mut().find(|(key, _)| *key == id) {
                items.push(item);
            } else {
                groups.push((id, vec![item]));
            }
        }
        for (_, items) in groups {
            for item in items.into_iter().skip(1) {
                let (line, col) = source.offset_to_line_col(item.start_byte());
                let mut d = self.diagnostic(
                    source,
                    line,
                    col,
                    format!(
                        "Duplicate value `{}` found in `{name}` enum declaration.",
                        node_text(source, item)
                    ),
                );
                d.highlight_length =
                    super::blank::highlight_length(source, item.start_byte(), item.end_byte());
                diagnostics.push(d);
            }
        }
    }
}
impl Cop for EnumUniqueness {
    fn name(&self) -> &'static str {
        "Rails/EnumUniqueness"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/models/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"enum"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"enum")
            || call_receiver(node).is_some()
            || node.child_by_field_name("operator").is_some()
        {
            return;
        }
        let args = argument_nodes(node);
        if args.len() >= 2 && collection(args[1]) {
            self.report(source, args[0], args[1], diagnostics);
        }
        let pairs = if let [options] = args.as_slice() {
            if options.kind() == "hash" {
                named_kids(*options)
            } else {
                args.clone()
            }
        } else if args
            .iter()
            .all(|n| matches!(n.kind(), "pair" | "hash_splat_argument"))
        {
            args
        } else {
            vec![]
        };
        for pair in pairs {
            if let (Some(key), Some(value)) = (
                pair.child_by_field_name("key"),
                pair.child_by_field_name("value"),
            ) {
                if collection(value) {
                    self.report(source, key, value, diagnostics);
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnumUniqueness, "cops/rails/enum_uniqueness");
}
