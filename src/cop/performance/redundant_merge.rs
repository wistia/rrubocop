use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::ruby_semantics::{pure, value_used};
use crate::cop::shared::{
    call_method_name, call_receiver, named_kids, node_bytes, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn accumulator(source: &SourceFile, node: Node<'_>, mut receiver: Node<'_>) -> bool {
    while receiver.kind() == "call" || receiver.kind() == "element_reference" {
        let Some(next) = call_receiver(receiver).or_else(|| receiver.child_by_field_name("object"))
        else {
            return false;
        };
        receiver = next;
    }
    if receiver.kind() != "identifier" || !crate::cop::ported::local_identifier(source, receiver) {
        return false;
    }
    let Some(mut parent) = node.parent() else {
        return false;
    };
    if matches!(parent.kind(), "body_statement" | "block_body") {
        let Some(next) = parent.parent() else {
            return false;
        };
        parent = next;
    }
    if !matches!(parent.kind(), "block" | "do_block") {
        return false;
    }
    let Some(call) = parent.parent() else {
        return false;
    };
    if call_method_name(source, call) != Some(b"each_with_object") || arguments(call).len() != 1 {
        return false;
    }
    let Some(params) = parent.child_by_field_name("parameters") else {
        return false;
    };
    let params = named_kids(params);
    params.len() == 2 && node_bytes(source, params[1]) == node_bytes(source, receiver)
}

fn maximum(config: &CopConfig) -> i128 {
    let Some(value) = config.options.get("MaxKeyValuePairs") else {
        return 2;
    };
    if value.is_null() || value.as_bool() == Some(false) {
        return 2;
    }
    if let Some(value) = value.as_i64() {
        return value as i128;
    }
    if let Some(value) = value.as_f64() {
        return value as i128;
    }
    if let Some(value) = value.as_str() {
        return value.parse().unwrap_or(2);
    }
    2
}
pub struct RedundantMerge;
impl Cop for RedundantMerge {
    fn name(&self) -> &'static str {
        "Performance/RedundantMerge"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"merge!"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"merge!")
            || node
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
        {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        let args = arguments(node);
        if args.len() != 1 || !args[0].hash {
            return;
        }
        let pairs = args[0].parts();
        if pairs.is_empty()
            || (pairs.len() as i128) > maximum(config)
            || pairs.iter().any(|p| p.kind() != "pair")
            || pairs.len() > 1 && !pure(source, receiver)
            || value_used(source, node) && !accumulator(source, node, receiver)
        {
            return;
        }
        let mut assignments = Vec::new();
        let mut simple = true;
        for pair in &pairs {
            let Some(key) = pair.child_by_field_name("key") else {
                return;
            };
            let value = pair.child_by_field_name("value").unwrap_or(key);
            let raw = node_text(source, key);
            let colon = key.kind() == "hash_key_symbol"
                || node_bytes(source, *pair).get(key.end_byte() - pair.start_byte()) == Some(&b':');
            let key_source = if colon
                && crate::cop::shared::ruby_expression::plain_symbol(source, key).is_some()
            {
                format!(":{raw}")
            } else {
                raw
            };
            assignments.push(format!(
                "{}[{}] = {}",
                node_text(source, receiver),
                key_source,
                node_text(source, value)
            ));
            simple &= matches!(key.kind(), "simple_symbol" | "hash_key_symbol" | "integer")
                && matches!(value.kind(), "integer" | "true" | "false" | "nil");
        }
        let end = super::send_range::end(node);
        let current = String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end]);
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            col,
            format!("Use `{}` instead of `{current}`.", assignments.join("; ")),
        );
        diagnostic.correctable = !config.autocorrect_disabled();
        diagnostic.highlight_length = current.split('\n').next().unwrap_or("").chars().count();
        // Literal single-pair receivers avoid unknown merge!/[]= implementations and RHS ordering changes.
        if !config.autocorrect_disabled()
            && receiver.kind() == "hash"
            && pairs.len() == 1
            && simple
            && !source.as_bytes().windows(7).any(|s| s == b"__LINE_")
        {
            diagnostic.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                end,
                assignments.remove(0),
                self.name(),
            );
        }
        diagnostics.push(diagnostic);
    }
}
