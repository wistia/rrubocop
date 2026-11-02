//! Require timestamp columns unless a table opts out of its primary key.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, for_each_descendant, named_kids, node_bytes,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct CreateTableWithTimestamps;
fn id_false(source: &SourceFile, node: Node<'_>) -> bool {
    let mut found = false;
    for_each_descendant(node, |n| {
        if n.kind() != "pair" {
            return;
        }
        if let (Some(key), Some(value)) =
            (n.child_by_field_name("key"), n.child_by_field_name("value"))
        {
            found |= value.kind() == "false"
                && crate::cop::ported::static_symbol(source, key).as_deref() == Some("id");
        }
    });
    found
}
fn time_columns(source: &SourceFile, body: Node<'_>) -> bool {
    let mut found = false;
    for_each_descendant(body, |n| {
        if crate::cop::ported::bare_call_identifier(source, n)
            && node_bytes(source, n) == b"timestamps"
        {
            found = true;
        }
        if crate::cop::shared::is_setter_call(n)
            || n.kind() != "call"
            || n.child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
        {
            return;
        }
        if call_method_name(source, n) == Some(b"timestamps") {
            found = true;
        }
        if call_method_name(source, n) == Some(b"datetime") {
            found |= argument_nodes(n).first().is_some_and(|arg| {
                matches!(
                    crate::cop::ported::static_literal(source, *arg).as_deref(),
                    Some("created_at" | "updated_at")
                )
            });
        }
    });
    found
}
impl Cop for CreateTableWithTimestamps {
    fn name(&self) -> &'static str {
        "Rails/CreateTableWithTimestamps"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["db/**/*.rb"]
    }
    fn default_exclude(&self) -> &'static [&'static str] {
        &[
            "**/db/**/*_create_active_storage_tables.active_storage.rb",
            "**/db/**/*_create_active_storage_variant_records.active_storage.rb",
        ]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"create_table"]
    }
    fn interested_identifier_names(&self) -> &'static [&'static [u8]] {
        &[b"create_table"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if node.kind() == "identifier" {
            if node_bytes(source, node) != b"create_table"
                || !crate::cop::ported::bare_call_identifier(source, node)
            {
                return;
            }
        } else if call_method_name(source, node) != Some(b"create_table")
            || call_receiver(node).is_some()
        {
            return;
        }
        let args = argument_nodes(node);
        if args.iter().any(|arg| id_false(source, *arg)) {
            return;
        }
        if args
            .first()
            .is_some_and(|n| crate::cop::ported::static_symbol(source, *n).is_some())
            && args.last().is_some_and(|arg| {
                arg.kind() == "block_argument"
                    && named_kids(*arg).first().is_some_and(|n| {
                        crate::cop::ported::static_symbol(source, *n).as_deref()
                            == Some("timestamps")
                    })
            })
        {
            return;
        }
        let block = node.child_by_field_name("block");
        let matching_block = block.filter(|b| {
            b.child_by_field_name("parameters")
                .is_some_and(|parameters| {
                    let args = named_kids(parameters);
                    args.len() == 1 && args[0].kind() == "identifier"
                })
        });
        if matching_block
            .and_then(|b| b.child_by_field_name("body"))
            .is_some_and(|body| time_columns(source, body))
        {
            return;
        }
        let end = if matching_block.is_some() {
            node.end_byte()
        } else {
            crate::cop::shared::send_end(node)
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Add timestamps when creating a new table.".into(),
        );
        d.highlight_length = super::blank::highlight_length(source, node.start_byte(), end);
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        CreateTableWithTimestamps,
        "cops/rails/create_table_with_timestamps"
    );
}
