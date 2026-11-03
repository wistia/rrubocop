use crate::cop::shared::ruby_arguments::{Argument, arguments};
use crate::cop::shared::ruby_expression::{
    plain_string, plain_symbol, regular_send_name, regular_send_receiver,
};
use crate::cop::shared::{call_method_name, named_kids, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn key(source: &SourceFile, pair: Node<'_>, name: &str) -> bool {
    pair.kind() == "pair"
        && pair
            .child_by_field_name("key")
            .and_then(|key| plain_symbol(source, key))
            .as_deref()
            == Some(name)
}
fn null_false<'n>(source: &SourceFile, options: &Argument<'n>) -> Option<Node<'n>> {
    if !options.hash {
        return None;
    }
    let pairs = options.parts();
    if pairs.iter().any(|pair| {
        key(source, *pair, "default")
            && !pair
                .child_by_field_name("value")
                .is_some_and(|v| v.kind() == "nil")
    }) {
        return None;
    }
    pairs.into_iter().find(|pair| {
        key(source, *pair, "null")
            && pair
                .child_by_field_name("value")
                .is_some_and(|value| value.kind() == "false")
    })
}
fn exempt_type(source: &SourceFile, node: Node<'_>, database: Option<&str>) -> bool {
    let value = plain_symbol(source, node).or_else(|| plain_string(source, node));
    value.as_deref() == Some("virtual")
        || database == Some("mysql") && value.as_deref() == Some("text")
}
fn change_table_pairs<'n>(
    source: &SourceFile,
    node: Node<'n>,
    database: Option<&str>,
) -> Vec<Node<'n>> {
    let Some(block) = node.child_by_field_name("block") else {
        return Vec::new();
    };
    let Some(parameters) = block.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let params = named_kids(parameters)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect::<Vec<_>>();
    if params.len() != 1 || params[0].kind() != "identifier" {
        return Vec::new();
    }
    let Some(body) = block.child_by_field_name("body") else {
        return Vec::new();
    };
    let mut statements = if matches!(body.kind(), "body_statement" | "block_body") {
        named_kids(body)
    } else {
        vec![body]
    };
    statements.retain(|n| n.kind() != "comment");
    if statements.len() == 1 && statements[0].kind() == "parenthesized_statements" {
        statements = named_kids(statements[0]);
    }
    let mut pairs = Vec::new();
    for statement in statements {
        if statement.child_by_field_name("block").is_some() {
            continue;
        }
        let Some(Some(receiver)) = regular_send_receiver(source, statement) else {
            continue;
        };
        if receiver.kind() != "identifier"
            || !crate::cop::ported::local_identifier(source, receiver)
            || node_bytes(source, receiver) != node_bytes(source, params[0])
        {
            continue;
        }
        let Some(method) = regular_send_name(source, statement) else {
            continue;
        };
        let args = arguments(statement);
        if method.as_ref() == b"column"
            && args.len() == 3
            && !args[1]
                .node()
                .is_some_and(|t| exempt_type(source, t, database))
        {
            pairs.extend(null_false(source, &args[2]));
        }
        if args.len() == 2 {
            pairs.extend(null_false(source, &args[1]));
        }
        if method.as_ref() == b"add_reference" && args.len() == 3 {
            pairs.extend(null_false(source, &args[2]));
        }
    }
    pairs
}
pub struct NotNullColumn;
impl Cop for NotNullColumn {
    fn name(&self) -> &'static str {
        "Rails/NotNullColumn"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["db/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"add_column", b"add_reference", b"change_table"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if regular_send_receiver(source, node) != Some(None) {
            return;
        }
        let database = super::database::database(source, config);
        let method = call_method_name(source, node);
        let args = arguments(node);
        let mut candidates = Vec::new();
        if method == Some(b"add_column")
            && args.len() == 4
            && !args[2]
                .node()
                .is_some_and(|t| exempt_type(source, t, database.as_deref()))
        {
            candidates.extend(null_false(source, &args[3]));
        }
        if method == Some(b"add_reference") && args.len() == 3 {
            candidates.extend(null_false(source, &args[2]));
        }
        if method == Some(b"change_table") {
            candidates.extend(change_table_pairs(source, node, database.as_deref()));
        }
        for pair in candidates {
            let (line, column) = source.offset_to_line_col(pair.start_byte());
            if diagnostics.iter().any(|d| {
                d.cop_name == self.name() && d.location.line == line && d.location.column == column
            }) {
                continue;
            }
            let mut d = self.diagnostic(
                source,
                line,
                column,
                "Do not add a NOT NULL column without a default value.".into(),
            );
            d.severity = config.severity.unwrap_or(d.severity);
            d.highlight_length =
                String::from_utf8_lossy(&source.as_bytes()[pair.start_byte()..pair.end_byte()])
                    .split('\n')
                    .next()
                    .unwrap_or("")
                    .chars()
                    .count();
            diagnostics.push(d);
        }
    }
}
