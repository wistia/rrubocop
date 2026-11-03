//! GraphQL/PrepareMethod — prefer prepare: :method over lambdas/consts.

use tree_sitter::Node;

use super::helpers::{CALL_KINDS, DEPT_INCLUDE, bare_method};
use crate::cop::shared::node_text;
use crate::cop::shared::{ruby_arguments, ruby_expression::plain_symbol};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PrepareMethod;

impl Cop for PrepareMethod {
    fn name(&self) -> &'static str {
        "GraphQL/PrepareMethod"
    }

    fn default_include(&self) -> &'static [&'static str] {
        DEPT_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        CALL_KINDS
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        super::helpers::ARGUMENT_METHODS
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !(bare_method(source, node, b"argument")
            || bare_method(source, node, b"public_argument"))
        {
            return;
        }
        let Some(prep) = last_prepare_value(source, node) else {
            return;
        };
        let style = config.get_str("EnforcedStyle", "");
        if block_or_constant(prep) {
            diagnostics.push(at_value(self, source, prep, lambda_msg(style).into()));
            return;
        }
        if let Some(msg) = style_mismatch_msg(source, prep, style) {
            diagnostics.push(at_value(self, source, prep, msg));
        }
    }
}

fn last_prepare_value<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let arguments = ruby_arguments::arguments(node);
    let hash = arguments.last().filter(|arg| arg.hash)?;
    let pair = hash
        .parts()
        .last()
        .copied()
        .filter(|node| node.kind() == "pair")?;
    let key = pair.child_by_field_name("key")?;
    if plain_symbol(source, key).as_deref() != Some("prepare") {
        return None;
    }
    pair.child_by_field_name("value")
}

fn at_value(
    cop: &PrepareMethod,
    source: &SourceFile,
    node: Node<'_>,
    message: String,
) -> Diagnostic {
    let (line, column) = source.offset_to_line_col(node.start_byte());
    let mut diagnostic = cop.diagnostic(source, line, column, message);
    let (last_line, last_column) = source.offset_to_line_col(node.end_byte());
    let end_column = if last_line == line {
        last_column
    } else {
        source
            .line_text(line)
            .map_or(column, |text| text.chars().count())
    };
    diagnostic.highlight_length = end_column.saturating_sub(column).max(1);
    diagnostic
}

fn block_or_constant(prep: Node<'_>) -> bool {
    matches!(
        prep.kind(),
        "block" | "do_block" | "lambda" | "constant" | "scope_resolution"
    ) || prep.child_by_field_name("block").is_some()
}

fn lambda_msg(style: &str) -> &'static str {
    match style {
        "symbol" => "Avoid using prepare lambdas, use prepare: :method_name instead.",
        "string" => "Avoid using prepare lambdas, use prepare: \"method_name\" instead.",
        _ => {
            "Avoid using prepare lambdas, use prepare: :method_name or prepare: \"method_name\" instead."
        }
    }
}

fn style_mismatch_msg(source: &SourceFile, prep: Node<'_>, style: &str) -> Option<String> {
    let kind = prep.kind();
    if style == "symbol" && matches!(kind, "string" | "string_content" | "interpolated_string") {
        let name = node_text(source, prep)
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        Some(format!(
            "Avoid using prepare string, use prepare: :{name} instead."
        ))
    } else if style == "string" && matches!(kind, "simple_symbol" | "symbol" | "hash_key_symbol") {
        let name = node_text(source, prep).trim_start_matches(':').to_string();
        Some(format!(
            "Avoid using prepare symbols, use prepare: \"{name}\" instead."
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_final_hash_pair_and_production_regressions() {
        crate::testutil::assert_original_spec_parity(
            &super::PrepareMethod,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/production-matchers/graphql-prepare.json"
            )),
        );
    }
}
