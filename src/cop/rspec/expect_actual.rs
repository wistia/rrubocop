//! RSpec/ExpectActual — `expect` must not receive a literal actual.

use tree_sitter::Node;

use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::shared::{argument_nodes, call_method_name, call_receiver};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ExpectActual;

const SKIP_MATCHERS: &[&[u8]] = &[b"route_to", b"be_routable"];

const SIMPLE: &[&str] = &[
    "true",
    "false",
    "nil",
    "integer",
    "float",
    "string",
    "simple_symbol",
    "symbol",
];

fn complex_literal(node: Node<'_>) -> bool {
    if !matches!(node.kind(), "array" | "hash" | "pair" | "range" | "regex") {
        return false;
    }
    let mut cur = node.walk();
    node.named_children(&mut cur).all(is_literal)
}

fn is_literal(node: Node<'_>) -> bool {
    if node.kind() == "string" {
        let mut cursor = node.walk();
        return !node
            .named_children(&mut cursor)
            .any(|n| n.kind() == "interpolation");
    }
    SIMPLE.contains(&node.kind()) || complex_literal(node)
}

fn matcher_name<'a>(source: &'a SourceFile, runner: Node<'_>) -> Option<&'a [u8]> {
    let first = argument_nodes(runner).into_iter().next()?;
    match first.kind() {
        "call" | "command" => call_method_name(source, first),
        "identifier" => Some(crate::cop::shared::node_bytes(source, first)),
        _ => None,
    }
}

fn expect_actual_arg<'a>(source: &SourceFile, runner: Node<'a>) -> Option<Node<'a>> {
    if argument_nodes(runner).len() != 1 {
        return None;
    }
    let recv = call_receiver(runner)?;
    if call_method_name(source, recv) != Some(b"expect") {
        return None;
    }
    let actual = argument_nodes(recv).into_iter().next()?;
    is_literal(actual).then_some(actual)
}

impl Cop for ExpectActual {
    fn name(&self) -> &'static str {
        "RSpec/ExpectActual"
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &["**/spec/routing/**/*"]
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"to", b"not_to", b"to_not"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(actual) = expect_actual_arg(source, node) else {
            return;
        };
        if matcher_name(source, node).is_some_and(|m| SKIP_MATCHERS.contains(&m)) {
            return;
        }
        let (line, col) = source.offset_to_line_col(actual.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "Provide the actual value you are testing to `expect(...)`.".into(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectActual, "cops/rspec/expect_actual");
}
