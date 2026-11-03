use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::ruby_expression::{plain_symbol, regular_send_name};
use crate::cop::shared::{call_method_name, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

const ALTER: &[&[u8]] = &[
    b"add_column",
    b"remove_column",
    b"remove_columns",
    b"change_column",
    b"add_timestamps",
    b"remove_timestamps",
];
const TRANSFORM: &[&[u8]] = &[
    b"primary_key",
    b"column",
    b"string",
    b"text",
    b"integer",
    b"bigint",
    b"float",
    b"decimal",
    b"numeric",
    b"datetime",
    b"timestamp",
    b"time",
    b"date",
    b"binary",
    b"boolean",
    b"json",
    b"virtual",
    b"remove",
    b"change",
    b"timestamps",
    b"remove_timestamps",
];
fn combinable(method: &[u8], mysql: bool, version: f64, alter: bool) -> bool {
    if if alter {
        ALTER.contains(&method)
    } else {
        TRANSFORM.contains(&method)
    } {
        return true;
    }
    if mysql {
        return if alter {
            matches!(method, b"rename_column" | b"add_index" | b"remove_index")
        } else {
            matches!(method, b"rename" | b"index" | b"remove_index")
        };
    }
    if alter {
        method == b"change_column_default" || version >= 6.1 && method == b"change_column_null"
    } else {
        method == b"change_default" || version >= 6.1 && method == b"change_null"
    }
}
fn table(source: &SourceFile, node: Node<'_>) -> Option<String> {
    arguments(node)
        .first()?
        .node()
        .and_then(|n| super::literal_value::value(source, n))
}
fn push(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
    message: String,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, column) = source.offset_to_line_col(node.start_byte());
    let mut d = cop.diagnostic(source, line, column, message);
    d.severity = config.severity.unwrap_or(d.severity);
    let end = crate::cop::shared::send_end(node);
    d.highlight_length = super::blank::highlight_length(source, node.start_byte(), end);
    diagnostics.push(d);
}
pub struct BulkChangeTable;
impl Cop for BulkChangeTable {
    fn name(&self) -> &'static str {
        "Rails/BulkChangeTable"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["db/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier", "method"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let database = super::database::database(source, config);
        let mysql = database.as_deref() == Some("mysql");
        let version = config.get_f64("TargetRailsVersion", 5.0);
        if !mysql && !(database.as_deref() == Some("postgresql") && version >= 5.2) {
            return;
        }
        if node.kind() == "method" {
            if !node
                .child_by_field_name("name")
                .is_some_and(|name| matches!(node_bytes(source, name), b"change" | b"up" | b"down"))
            {
                return;
            }
            let Some(body) = node.child_by_field_name("body") else {
                return;
            };
            let mut run = Vec::new();
            let mut previous = None;
            let mut starts = Vec::new();
            for (child, send) in super::migration_body::body_children(source, body) {
                let method = regular_send_name(source, child);
                let current = if send
                    && method
                        .as_deref()
                        .is_some_and(|method| combinable(method, mysql, version, true))
                {
                    table(source, child)
                } else {
                    None
                };
                if current.is_none()
                    || previous
                        .as_ref()
                        .is_some_and(|previous| Some(previous) != current.as_ref())
                {
                    if run.len() > 1 {
                        starts.push(run[0]);
                    }
                    run.clear();
                }
                if current.is_some() {
                    run.push(child);
                }
                previous = current;
            }
            if run.len() > 1 {
                starts.push(run[0]);
            }
            for first in starts {
                let value = table(source, first).unwrap();
                push(
                    self,
                    source,
                    first,
                    config,
                    format!(
                        "You can use `change_table :{value}, bulk: true` to combine alter queries."
                    ),
                    diagnostics,
                );
            }
            return;
        }
        if call_method_name(source, node) != Some(b"change_table")
            || node.child_by_field_name("receiver").is_some()
            || node
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
        {
            return;
        }
        let args = arguments(node);
        if args.get(1).is_some_and(|a| {
            a.hash
                && a.parts().iter().any(|pair| {
                    pair.child_by_field_name("key")
                        .and_then(|key| plain_symbol(source, key))
                        .as_deref()
                        == Some("bulk")
                })
        }) {
            return;
        }
        let Some(body) = node
            .child_by_field_name("block")
            .and_then(|b| b.child_by_field_name("body"))
        else {
            return;
        };
        let count: usize = super::migration_body::body_sends(source, body)
            .iter()
            .map(|n| {
                let method = regular_send_name(source, *n);
                if method.as_deref() == Some(b"remove") {
                    arguments(*n).iter().filter(|a| !a.hash).count()
                } else {
                    usize::from(
                        method
                            .as_deref()
                            .is_some_and(|m| combinable(m, mysql, version, false)),
                    )
                }
            })
            .sum();
        if count > 1 {
            push(
                self,
                source,
                node,
                config,
                "You can combine alter queries using `bulk: true` options.".into(),
                diagnostics,
            );
        }
    }
}
