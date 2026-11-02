//! Style/DateTime.

use tree_sitter::Node;

use crate::cop::shared::{
    call_method_name, call_receiver, node_bytes, push_replace,
    semantic_argument_nodes as argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DateTime;

fn root_date_time(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(node.kind(), "constant" | "scope_resolution")
        && matches!(node_bytes(source, node), b"DateTime" | b"::DateTime")
}

fn historical_date(source: &SourceFile, call: Node<'_>) -> bool {
    let arguments = argument_nodes(call);
    if arguments.len() != 2
        || call
            .child_by_field_name("operator")
            .is_some_and(|operator| node_bytes(source, operator) == b"&.")
    {
        return false;
    }
    let date = arguments[1];
    date.kind() == "scope_resolution"
        && date
            .child_by_field_name("scope")
            .is_some_and(|scope| matches!(node_bytes(source, scope), b"Date" | b"::Date"))
}

impl Cop for DateTime {
    fn name(&self) -> &'static str {
        "Style/DateTime"
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

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let coercion = call_method_name(source, node) == Some(b"to_datetime")
            && argument_nodes(node).is_empty();
        let receiver = call_receiver(node).filter(|receiver| root_date_time(source, *receiver));
        if !(receiver.is_some() || (coercion && !config.get_bool("AllowCoercion", false)))
            || historical_date(source, node)
        {
            return;
        }
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            if coercion {
                "Do not use `#to_datetime`.".into()
            } else {
                "Prefer `Time` over `DateTime`.".into()
            },
        );
        let mut end = node
            .child_by_field_name("block")
            .map_or(node.end_byte(), |block| block.start_byte());
        while end > node.start_byte() && source.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                .chars()
                .count();
        if coercion {
            diagnostic.correctable = false;
        } else if let Some(receiver) = receiver {
            let name = receiver.child_by_field_name("name").unwrap_or(receiver);
            diagnostic.corrected = push_replace(
                &mut corrections,
                name.start_byte(),
                name.end_byte(),
                "Time",
                self.name(),
            );
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &DateTime,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/date_time/parity.json"
            )),
        );
    }
}
