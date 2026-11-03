use crate::cop::shared::ruby_arguments::{Argument, arguments};
use crate::cop::shared::ruby_expression::{
    plain_string, plain_symbol, regular_send_name, regular_send_receiver,
};
use crate::cop::shared::{call_method_name, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn ancestors(node: Node<'_>) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    let mut parent = node.parent();
    while let Some(n) = parent {
        result.push(n);
        parent = n.parent();
    }
    result
}
fn enabled(source: &SourceFile, node: Node<'_>, ruby_version: f64) -> bool {
    let parents = ancestors(node);
    parents
        .iter()
        .any(|n| n.kind() == "class" && crate::cop::ported::migration_class(source, *n))
        && parents.iter().any(|n| {
            n.kind() == "method"
                && n.child_by_field_name("name")
                    .is_some_and(|n| node_bytes(source, n) == b"change")
        })
        && !parents.iter().any(|n| {
            n.kind() == "call"
                && n.child_by_field_name("block").is_some_and(|block| {
                    ruby_version < 2.7
                        || block.child_by_field_name("parameters").is_some()
                        || crate::cop::shared::ruby_expression::numbered_bindings(source, block)
                            .is_none()
                })
                && matches!(
                    call_method_name(source, *n),
                    Some(b"reversible" | b"up_only")
                )
        })
}
fn keys(source: &SourceFile, arg: Option<&Argument<'_>>, wanted: &[&str]) -> bool {
    let Some(arg) = arg.filter(|arg| arg.hash) else {
        return false;
    };
    let names = arg
        .parts()
        .into_iter()
        .filter_map(|pair| pair.child_by_field_name("key"))
        .filter_map(|key| plain_symbol(source, key).or_else(|| plain_string(source, key)))
        .collect::<Vec<_>>();
    wanted
        .iter()
        .all(|wanted| names.iter().any(|name| name == wanted))
}
fn reversible(source: &SourceFile, node: Node<'_>, version: f64) -> bool {
    let method = regular_send_name(source, node);
    let args = arguments(node);
    match method.as_deref() {
        Some(b"change") => false,
        Some(b"remove") => version >= 6.1 && keys(source, args.last(), &["type"]),
        Some(
            b"change_default"
            | b"change_column_default"
            | b"change_table_comment"
            | b"change_column_comment",
        ) => keys(source, args.last(), &["from", "to"]),
        _ => true,
    }
}
pub struct ReversibleMigration;
impl Cop for ReversibleMigration {
    fn name(&self) -> &'static str {
        "Rails/ReversibleMigration"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["db/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !enabled(source, node, config.get_f64("TargetRubyVersion", 3.4)) {
            return;
        }
        let version = config.get_f64("TargetRailsVersion", 5.0);
        let args = arguments(node);
        let method_name = regular_send_name(source, node);
        let method = method_name.as_deref();
        let mut findings = Vec::new();
        if regular_send_receiver(source, node) == Some(None) {
            match method {
                Some(b"change_column" | b"execute") => {
                    findings.push((node, String::from_utf8_lossy(method.unwrap()).into_owned()))
                }
                Some(b"drop_table")
                    if !node.child_by_field_name("block").is_some_and(|block| {
                        config.get_f64("TargetRubyVersion", 3.4) < 2.7
                            || crate::cop::shared::ruby_expression::numbered_bindings(source, block)
                                .is_none()
                    }) && !args
                        .last()
                        .and_then(|a| a.node())
                        .is_some_and(|n| n.kind() == "block_argument") =>
                {
                    findings.push((node, "drop_table(without block)".into()))
                }
                Some(b"remove_column") if args.len() < 3 => {
                    findings.push((node, "remove_column(without type)".into()))
                }
                Some(b"remove_foreign_key")
                    if args.len() == 2
                        && args[1].hash
                        && !keys(source, Some(&args[1]), &["to_table"]) =>
                {
                    findings.push((node, "remove_foreign_key(without table)".into()))
                }
                Some(b"remove_columns")
                    if !args.is_empty()
                        && !(version >= 6.1 && keys(source, args.last(), &["type"])) =>
                {
                    findings.push((
                        node,
                        if version >= 6.1 {
                            "remove_columns(without type)"
                        } else {
                            "remove_columns"
                        }
                        .into(),
                    ))
                }
                Some(b"remove_index")
                    if args.len() == 2
                        && args[1].hash
                        && !keys(source, Some(&args[1]), &["column"]) =>
                {
                    findings.push((node, "remove_index(without column)".into()))
                }
                _ => {}
            }
        }
        if !reversible(source, node, version) {
            findings.push((
                node,
                format!(
                    "{}(without :from and :to)",
                    String::from_utf8_lossy(method.unwrap_or_default())
                ),
            ));
        }
        if method == Some(b"change_table")
            && regular_send_receiver(source, node) == Some(None)
            && !args.is_empty()
        {
            if let Some(body) = node
                .child_by_field_name("block")
                .and_then(|b| b.child_by_field_name("body"))
            {
                let table = args[0].node();
                for child in super::migration_body::body_sends(source, body) {
                    let receiver = regular_send_receiver(source, child).flatten();
                    let same = match (table, receiver) {
                        (Some(a), Some(b)) => {
                            crate::cop::shared::ruby_expression::same_expression(source, a, b)
                        }
                        _ => false,
                    };
                    if !same && reversible(source, child, version) {
                        continue;
                    }
                    let child_method = regular_send_name(source, child).unwrap();
                    let action = if child_method.as_ref() == b"remove" {
                        if version >= 6.1 {
                            "t.remove (without type)".into()
                        } else {
                            "t.remove".into()
                        }
                    } else {
                        format!(
                            "change_table(with {})",
                            String::from_utf8_lossy(&child_method)
                        )
                    };
                    findings.push((child, action));
                }
            }
        }
        for (target, action) in findings {
            let (line, column) = source.offset_to_line_col(target.start_byte());
            let message = format!("{action} is not reversible.");
            if diagnostics.iter().any(|d| {
                d.cop_name == self.name() && d.location.line == line && d.location.column == column
            }) {
                continue;
            }
            let mut d = self.diagnostic(source, line, column, message);
            d.severity = config.severity.unwrap_or(d.severity);
            let end = super::super::shared::send_end(target);
            d.highlight_length = super::blank::highlight_length(source, target.start_byte(), end);
            diagnostics.push(d);
        }
    }
}
