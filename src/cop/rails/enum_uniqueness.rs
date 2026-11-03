//! Report every repeated enum value after the first, grouped in original value order.
use crate::cop::shared::ruby_expression::eql_expression;
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
fn grouped_values<'a>(source: &SourceFile, elements: Vec<Node<'a>>) -> Vec<Vec<Node<'a>>> {
    let mut groups: Vec<Vec<Node<'_>>> = vec![];
    let mut integers = std::collections::HashMap::<num_bigint::BigInt, Vec<usize>>::new();
    let mut opaque = Vec::new();
    for item in elements {
        let integer = crate::cop::shared::ruby_expression::integer_ast(source, item);
        let matching = if let Some(integer) = &integer {
            integers
                .get(integer)
                .into_iter()
                .flatten()
                .chain(&opaque)
                .copied()
                .filter(|&index| eql_expression(source, groups[index][0], item))
                .min()
        } else {
            groups
                .iter()
                .position(|items| eql_expression(source, items[0], item))
        };
        if let Some(index) = matching {
            groups[index].push(item);
        } else {
            let index = groups.len();
            if let Some(integer) = integer {
                integers.entry(integer).or_default().push(index);
            } else {
                opaque.push(index);
            }
            groups.push(vec![item]);
        }
    }
    groups
}
impl EnumUniqueness {
    fn report(
        &self,
        source: &SourceFile,
        key: Node<'_>,
        values: Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let name = crate::cop::shared::ruby_expression::plain_symbol(source, key)
            .or_else(|| crate::cop::shared::ruby_expression::plain_string(source, key))
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
        let groups = grouped_values(source, elements);
        for items in groups {
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

#[cfg(test)]
mod performance_tests {
    use super::*;
    #[test]
    fn integer_buckets_preserve_strict_groups_order_and_opaque_fallback() {
        let distinct = format!(
            "[{}]",
            (0..128)
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        for code in [
            distinct.as_str(),
            "[1, 1.0, 0x1, __LINE__, 1r, 1i, 1.0i, :x, 'x', :x, [1], [0x1], -1, -0x1, 1, {a: 1}, {a: 0x1}]",
        ] {
            let source = SourceFile::from_bytes("test.rb", code.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            let array = named_kids(tree.root_node())[0];
            let values = named_kids(array);
            let mut original: Vec<Vec<Node<'_>>> = Vec::new();
            for &item in &values {
                if let Some(group) = original
                    .iter_mut()
                    .find(|group| eql_expression(&source, group[0], item))
                {
                    group.push(item);
                } else {
                    original.push(vec![item]);
                }
            }
            assert_eq!(grouped_values(&source, values), original);
        }
    }
}
