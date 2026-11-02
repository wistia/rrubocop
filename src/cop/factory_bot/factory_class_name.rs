//! Keep factory class names lazy by passing the constant's source as a string.
use crate::cop::shared::{argument_nodes, call_method_name, named_kids, node_text, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct FactoryClassName;
impl Cop for FactoryClassName {
    fn name(&self) -> &'static str {
        "FactoryBot/FactoryClassName"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"factory"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"factory") {
            return;
        }
        let args = argument_nodes(node);
        if args.len() < 2 {
            return;
        }
        let options: Vec<_> = if args.len() == 2 && args[1].kind() == "hash" {
            named_kids(args[1])
        } else if args[1..].iter().all(|n| n.kind() == "pair") {
            args[1..].to_vec()
        } else {
            return;
        };
        for pair in options {
            if pair.kind() != "pair" {
                continue;
            }
            let Some(key) = pair.child_by_field_name("key") else {
                continue;
            };
            if !matches!(
                key.kind(),
                "hash_key_symbol" | "simple_symbol" | "delimited_symbol"
            ) || node_text(source, key)
                .trim_start_matches(':')
                .trim_matches(['\'', '"'])
                != "class"
            {
                continue;
            }
            let Some(value) = pair.child_by_field_name("value") else {
                continue;
            };
            if !matches!(value.kind(), "constant" | "scope_resolution") {
                continue;
            }
            let class = node_text(source, value);
            let name = class
                .trim_start_matches("::")
                .split_whitespace()
                .collect::<String>();
            if matches!(name.as_str(), "Hash" | "OpenStruct") {
                continue;
            }
            let (line, col) = source.offset_to_line_col(value.start_byte());
            let mut d = self.diagnostic(
                source,
                line,
                col,
                format!("Pass '{name}' string instead of `{name}` constant."),
            );
            d.highlight_length = class.chars().count();
            d.correctable = !config.autocorrect_disabled();
            d.corrected = push_replace(
                &mut corrections,
                value.start_byte(),
                value.start_byte(),
                "'",
                self.name(),
            );
            push_replace(
                &mut corrections,
                value.end_byte(),
                value.end_byte(),
                "'",
                self.name(),
            );
            diagnostics.push(d);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FactoryClassName, "cops/factory_bot/factory_class_name");
}
