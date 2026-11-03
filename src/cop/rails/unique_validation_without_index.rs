use crate::cop::shared::ruby_arguments::{Argument, arguments};
use crate::cop::shared::ruby_expression::{
    plain_string, plain_symbol, regular_send_arguments, regular_send_name, regular_send_receiver,
};
use crate::cop::shared::{call_method_name, named_kids, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

fn walk(node: Node<'_>) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    let mut pending = vec![node];
    while let Some(n) = pending.pop() {
        result.push(n);
        pending.extend(named_kids(n).into_iter().rev());
    }
    result
}
fn name(source: &SourceFile, node: Node<'_>) -> Option<String> {
    plain_symbol(source, node).or_else(|| plain_string(source, node))
}
fn option<'n>(source: &SourceFile, arg: &Argument<'n>, wanted: &str) -> Option<Node<'n>> {
    if !arg.hash {
        return None;
    }
    arg.parts()
        .into_iter()
        .find(|pair| {
            pair.child_by_field_name("key")
                .and_then(|key| plain_symbol(source, key))
                .as_deref()
                == Some(wanted)
        })
        .and_then(|pair| pair.child_by_field_name("value"))
}
fn value_option<'n>(source: &SourceFile, node: Node<'n>, wanted: &str) -> Option<Node<'n>> {
    if node.kind() != "hash" {
        return None;
    }
    let arg = Argument {
        nodes: vec![node],
        hash: true,
        braces: true,
    };
    option(source, &arg, wanted)
}
fn has_condition(source: &SourceFile, arg: &Argument<'_>, names: &[&str]) -> bool {
    arg.hash
        && arg.parts().iter().any(|pair| {
            pair.child_by_field_name("key")
                .and_then(|n| plain_symbol(source, n))
                .is_some_and(|key| names.iter().any(|name| key == *name))
        })
}
fn class_node(mut node: Node<'_>) -> Option<Node<'_>> {
    while let Some(parent) = node.parent() {
        if parent.kind() == "class" {
            return Some(parent);
        }
        node = parent;
    }
    None
}
fn const_parts(source: &SourceFile, node: Node<'_>, parts: &mut Vec<String>) {
    if node.kind() == "constant" {
        parts.push(String::from_utf8_lossy(node_bytes(source, node)).into_owned());
    } else if node.kind() == "scope_resolution" {
        if let Some(name) = node.child_by_field_name("name") {
            parts.push(String::from_utf8_lossy(node_bytes(source, name)).into_owned());
        }
        if let Some(scope) = node.child_by_field_name("scope") {
            const_parts(source, scope, parts);
        } else {
            parts.push(String::new());
        }
    }
}
fn table_name(source: &SourceFile, class: Node<'_>) -> Option<String> {
    struct TableName(Option<String>);
    source
        .cached(class.start_byte(), || {
            TableName(table_name_uncached(source, class))
        })
        .0
        .clone()
}
fn table_name_uncached(source: &SourceFile, class: Node<'_>) -> Option<String> {
    let mut overrides = Vec::new();
    for node in walk(class) {
        if regular_send_name(source, node).as_deref() != Some(b"table_name=") {
            continue;
        }
        if !regular_send_receiver(source, node)
            .flatten()
            .is_some_and(|r| r.kind() == "self")
        {
            continue;
        }
        if let Some(value) = regular_send_arguments(source, node)
            .filter(|args| args.len() == 1)
            .and_then(|args| name(source, args[0]))
        {
            overrides.push(value);
        }
    }
    if let Some(value) = overrides.pop() {
        return Some(value);
    }
    let mut parts = Vec::new();
    const_parts(source, class.child_by_field_name("name")?, &mut parts);
    let mut parent = class.parent();
    while let Some(node) = parent {
        if matches!(node.kind(), "class" | "module") {
            if let Some(identifier) = node.child_by_field_name("name") {
                let identifier = identifier.child_by_field_name("name").unwrap_or(identifier);
                parts.push(String::from_utf8_lossy(node_bytes(source, identifier)).into_owned());
            }
        }
        parent = node.parent();
    }
    parts.reverse();
    Some(crate::cop::shared::ruby_inflection::tableize(
        &parts.join("_"),
    ))
}
fn scope_names(source: &SourceFile, uniqueness: Node<'_>) -> Option<Vec<String>> {
    let mut scope = value_option(source, uniqueness, "scope")?;
    if scope.child_by_field_name("block").is_none()
        && regular_send_name(source, scope).as_deref() == Some(b"freeze")
    {
        scope = regular_send_receiver(source, scope).flatten()?;
    }
    if matches!(scope.kind(), "array" | "string_array" | "symbol_array") {
        named_kids(scope)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .map(|n| {
                if matches!(n.kind(), "bare_string" | "bare_symbol") {
                    crate::cop::shared::word_value(source, n, scope)
                } else {
                    name(source, n)
                }
            })
            .collect()
    } else {
        name(source, scope).map(|name| vec![name])
    }
}
fn resolve(
    source: &SourceFile,
    class: Node<'_>,
    table: &super::schema_model::Table,
    name: &str,
) -> Option<Vec<String>> {
    if table.columns.contains(name) {
        return Some(vec![name.into()]);
    }
    for relation in walk(class) {
        if regular_send_name(source, relation).as_deref() != Some(b"belongs_to")
            || regular_send_receiver(source, relation) != Some(None)
        {
            continue;
        }
        let args = arguments(relation);
        if args
            .first()
            .and_then(|arg| arg.node())
            .and_then(|arg| self::name(source, arg))
            .as_deref()
            != Some(name)
        {
            continue;
        }
        let key = args
            .last()
            .and_then(|arg| option(source, arg, "foreign_key"))
            .and_then(|n| self::name(source, n))
            .unwrap_or_else(|| format!("{name}_id"));
        if !table.columns.contains(&key) {
            continue;
        }
        let polymorphic = args
            .last()
            .and_then(|arg| option(source, arg, "polymorphic"))
            .is_some_and(|n| n.kind() == "true");
        return Some(if polymorphic {
            vec![key, format!("{name}_type")]
        } else {
            vec![key]
        });
    }
    None
}
pub struct UniqueValidationWithoutIndex;
impl Cop for UniqueValidationWithoutIndex {
    fn name(&self) -> &'static str {
        "Rails/UniqueValidationWithoutIndex"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["app/models/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"validates"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"validates")
            || node
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
        {
            return;
        }
        let args = arguments(node);
        let Some(options) = args.last() else {
            return;
        };
        let Some(uniqueness) = option(source, options, "uniqueness") else {
            return;
        };
        if matches!(uniqueness.kind(), "false" | "nil")
            || has_condition(source, options, &["if", "unless"])
        {
            return;
        }
        if uniqueness.kind() == "hash"
            && has_condition(
                source,
                &Argument {
                    nodes: vec![uniqueness],
                    hash: true,
                    braces: true,
                },
                &["if", "unless", "conditions"],
            )
        {
            return;
        }
        let Some(schema) = super::schema_model::load(source, config) else {
            return;
        };
        let Some(class) = class_node(node) else {
            return;
        };
        let Some(table_name) = table_name(source, class) else {
            return;
        };
        let Some(table) = schema.table(&table_name) else {
            return;
        };
        let Some(first) = args
            .first()
            .and_then(|a| a.node())
            .and_then(|n| name(source, n))
        else {
            return;
        };
        let mut columns = vec![first];
        columns.extend(scope_names(source, uniqueness).unwrap_or_default());
        let Some(columns) = columns
            .iter()
            .map(|name| resolve(source, class, table, name))
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        let names = columns.into_iter().flatten().collect::<BTreeSet<_>>();
        if table
            .indices
            .iter()
            .chain(schema.external_indices(&table_name))
            .any(|index| {
                index.unique
                    && (index.columns.as_ref() == Some(&names)
                        || index
                            .expression
                            .as_ref()
                            .is_some_and(|expr| names.iter().all(|name| expr.contains(name))))
            })
        {
            return;
        }
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            column,
            "Uniqueness validation should have a unique index on the database column.".into(),
        );
        d.severity = config.severity.unwrap_or(d.severity);
        let end = crate::cop::shared::send_end(node);
        d.highlight_length = super::blank::highlight_length(source, node.start_byte(), end);
        diagnostics.push(d);
    }
}
