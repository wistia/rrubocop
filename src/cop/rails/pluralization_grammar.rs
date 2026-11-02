//! Match ActiveSupport numeric extension names to literal numeric values.
use crate::cop::shared::ruby_expression::{float_value, integer_ast};
use crate::cop::shared::{
    call_method_name, call_receiver, method_node, node_text, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct PluralizationGrammar;
const SINGULAR: &[&str] = &[
    "second",
    "minute",
    "hour",
    "day",
    "week",
    "fortnight",
    "month",
    "year",
    "byte",
    "kilobyte",
    "megabyte",
    "gigabyte",
    "terabyte",
    "petabyte",
    "exabyte",
    "zettabyte",
];
fn float_text(value: f64) -> String {
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-Infinity"
        } else {
            "Infinity"
        }
        .into();
    }
    if value != 0.0 && (value.abs() < 0.0001 || value.abs() >= 1e15) {
        let text = format!("{value:e}");
        let (mantissa, exponent) = text.split_once('e').unwrap();
        let mantissa = if mantissa.contains('.') {
            mantissa.to_owned()
        } else {
            format!("{mantissa}.0")
        };
        return format!("{mantissa}e{:+03}", exponent.parse::<i32>().unwrap());
    }
    format!("{value:?}")
}
impl Cop for PluralizationGrammar {
    fn name(&self) -> &'static str {
        "Rails/PluralizationGrammar"
    }
    fn supports_autocorrect(&self) -> bool {
        true
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
        if !super::unknown_env::regular(node, source) || crate::cop::shared::is_setter_call(node) {
            return;
        }
        let Some(name) = call_method_name(source, node).and_then(|n| std::str::from_utf8(n).ok())
        else {
            return;
        };
        let plural = name.ends_with('s');
        let base = if plural {
            &name[..name.len() - 1]
        } else {
            name
        };
        if !SINGULAR.contains(&base) {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        let folded_minus = node.parent().filter(|parent| {
            parent.kind() == "unary"
                && parent.child_by_field_name("operand") == Some(node)
                && parent
                    .child_by_field_name("operator")
                    .is_some_and(|n| node_text(source, n) == "-")
                && matches!(receiver.kind(), "integer" | "float")
                && !node_text(source, receiver).starts_with(['-', '+'])
        });
        let (number, singular) = if let Some(value) = integer_ast(source, receiver) {
            let value = if folded_minus.is_some() {
                -value
            } else {
                value
            };
            let singular = value == 1.into() || value == (-1).into();
            (value.to_string(), singular)
        } else if let Some(value) = float_value(source, receiver) {
            let value = if folded_minus.is_some() {
                -value
            } else {
                value
            };
            (float_text(value), value.abs() == 1.0)
        } else {
            return;
        };
        if plural != singular {
            return;
        }
        let correct = if plural {
            base.to_owned()
        } else {
            format!("{base}s")
        };
        let Some(selector) = method_node(node) else {
            return;
        };
        let start = folded_minus.map_or(node.start_byte(), |n| n.start_byte());
        let (line, col) = source.offset_to_line_col(start);
        let mut d = self.diagnostic(source, line, col, format!("Prefer `{number}.{correct}`."));
        d.highlight_length = super::blank::highlight_length(source, start, send_end(node));
        d.correctable = !config.autocorrect_disabled();
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                selector.end_byte(),
                correct,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
