//! Style/ArgumentsForwarding — `...` for forwarded args, or anonymous `&` / `**`.

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArgumentsForwarding;

fn within_block(mut node: Node<'_>) -> bool {
    loop {
        if matches!(node.kind(), "block" | "do_block")
            || node.child_by_field_name("block").is_some()
        {
            return true;
        }
        let Some(parent) = node.parent() else {
            return false;
        };
        node = parent;
    }
}

fn legacy_block_forwarding(config: &CopConfig) -> bool {
    let mut parts = config.get_str("__RubocopVersion", "1.91.0").split('.');
    parts.next() == Some("1")
        && parts
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .is_some_and(|minor| minor <= 65)
}

pub(super) const COP: &str = "Style/ArgumentsForwarding";

mod anon;
mod kw;

pub(super) fn param_name<'a>(source: &'a SourceFile, node: Node<'a>) -> Option<&'a [u8]> {
    node.child_by_field_name("name")
        .map(|n| node_bytes(source, n))
        .or_else(|| {
            let mut cur = node.walk();
            node.named_children(&mut cur)
                .find(|n| n.kind() == "identifier")
                .map(|n| node_bytes(source, n))
        })
}

fn rest_and_block<'a>(source: &'a SourceFile, params: Node<'a>) -> Option<(&'a [u8], &'a [u8])> {
    let mut rest = None;
    let mut block = None;
    let mut cur = params.walk();
    for child in params.named_children(&mut cur) {
        match child.kind() {
            "splat_parameter" => rest = param_name(source, child),
            "block_parameter" => block = param_name(source, child),
            _ => {}
        }
    }
    Some((rest?, block?))
}

pub(super) fn arg_ident<'a>(source: &'a SourceFile, node: Node<'a>) -> Option<&'a [u8]> {
    let mut cur = node.walk();
    node.named_children(&mut cur)
        .find(|n| n.kind() == "identifier")
        .map(|n| node_bytes(source, n))
}

fn forwards_call(source: &SourceFile, node: Node<'_>, rest: &[u8], block: &[u8]) -> bool {
    if !matches!(node.kind(), "call" | "command" | "command_call") {
        return false;
    }
    let args = argument_nodes(node);
    let mut has_rest = false;
    let mut has_block = false;
    for arg in &args {
        match arg.kind() {
            "splat_argument" if arg_ident(source, *arg) == Some(rest) => has_rest = true,
            "block_argument" if arg_ident(source, *arg) == Some(block) => has_block = true,
            _ => {}
        }
    }
    has_rest && has_block
}

fn nested_scope(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "method" | "singleton_method" | "class" | "module" | "singleton_class"
    )
}

/// True when `rest` or `block` is read outside `call` (the forwarded splats themselves do not count).
fn params_used_outside(
    source: &SourceFile,
    body: Node<'_>,
    call: Node<'_>,
    rest: &[u8],
    block: &[u8],
) -> bool {
    let start = call.start_byte();
    let end = call.end_byte();
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        if nested_scope(node) {
            continue;
        }
        if param_ident_outside(source, node, rest, block, start, end) {
            return true;
        }
        let mut cur = node.walk();
        stack.extend(node.named_children(&mut cur));
    }
    false
}

fn param_ident_outside(
    source: &SourceFile,
    node: Node<'_>,
    rest: &[u8],
    block: &[u8],
    start: usize,
    end: usize,
) -> bool {
    if node.kind() != "identifier"
        || (start..end).contains(&node.start_byte())
        || method_ident(node)
    {
        return false;
    }
    let text = node_bytes(source, node);
    text == rest || text == block
}

fn method_ident(node: Node<'_>) -> bool {
    node.parent()
        .and_then(|parent| parent.child_by_field_name("method"))
        .is_some_and(|method| method.id() == node.id())
}

pub(super) fn method_body(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("body").or_else(|| {
        let mut cur = node.walk();
        node.named_children(&mut cur)
            .find(|n| n.kind() == "body_statement")
    })
}

fn find_forwarding_call(
    source: &SourceFile,
    body: Node<'_>,
    rest: &[u8],
    block: &[u8],
) -> Option<usize> {
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        if nested_scope(node) {
            continue;
        }
        if forwards_call(source, node, rest, block)
            && !params_used_outside(source, body, node, rest, block)
        {
            return Some(node.start_byte());
        }
        let mut cur = node.walk();
        stack.extend(node.named_children(&mut cur));
    }
    None
}

impl Cop for ArgumentsForwarding {
    fn name(&self) -> &'static str {
        COP
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["method", "singleton_method"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if legacy_block_forwarding(config) && within_block(node) {
            return;
        }
        let ruby = config.get_f64("TargetRubyVersion", 2.7);
        if ruby < 2.7 {
            return;
        }
        let Some(params) = node.child_by_field_name("parameters") else {
            return;
        };
        if config.get_bool("AllowOnlyRestArgument", true)
            && let Some((rest, block)) = rest_and_block(source, params)
            && let Some(body) = method_body(node)
            && let Some(off) = find_forwarding_call(source, body, rest, block)
        {
            let (line, col) = source.offset_to_line_col(off);
            let mut diag =
                self.diagnostic(source, line, col, "Use arguments forwarding.".to_string());
            diag.correctable = false;
            diagnostics.push(diag);
            return;
        }
        anon::report(
            self,
            source,
            node,
            params,
            config,
            ruby,
            diagnostics,
            &mut corrections,
        );
        kw::report(
            self,
            source,
            node,
            params,
            config,
            ruby,
            diagnostics,
            &mut corrections,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ArgumentsForwarding, "cops/style/arguments_forwarding");

    fn ruby34() -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".into(),
            serde_yml::Value::Number(serde_yml::Number::from(3.4)),
        );
        config
    }

    const LOADER: &str = "\
def load_variants(group_type, &block)
  return unless applicable?
  measure(operation:, group_type:, &block)
rescue StandardError
  raise
end
";

    fn collect(
        src: &str,
    ) -> (
        crate::parse::source::SourceFile,
        Vec<crate::correction::Correction>,
    ) {
        let source = crate::parse::source::SourceFile::from_bytes("t.rb", src.as_bytes().to_vec());
        let tree = crate::parse::parse_ruby(&source).expect("parse");
        let config = ruby34();
        let mut corrections = Vec::new();
        crate::cop::walker::BatchedWalker::new(vec![&ArgumentsForwarding], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut Vec::new(),
            Some(&mut corrections),
        );
        (source, corrections)
    }

    fn corrected(src: &str) -> String {
        let (source, corrections) = collect(src);
        String::from_utf8(
            crate::correction::CorrectionSet::from_vec(corrections).apply(source.as_bytes()),
        )
        .expect("utf8")
    }

    #[test]
    fn anonymous_block_is_correctable() {
        let diags = crate::testutil::run_cop_full_with_config(
            &ArgumentsForwarding,
            LOADER.as_bytes(),
            ruby34(),
        );
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| {
            d.correctable
                && d.highlight_length == 6
                && d.message == "Use anonymous block arguments forwarding (`&`)."
        }));
    }

    #[test]
    fn anonymous_block_autocorrect() {
        assert_eq!(
            corrected(LOADER),
            "\
def load_variants(group_type, &)
  return unless applicable?
  measure(operation:, group_type:, &)
rescue StandardError
  raise
end
"
        );
    }

    #[test]
    fn meaningful_block_name_is_clean() {
        let diags = crate::testutil::run_cop_full_with_config(
            &ArgumentsForwarding,
            b"def foo(&callback)\n  bar(&callback)\nend\ndef foo(&block)\n  block.call\nend\n",
            ruby34(),
        );
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn nested_def_does_not_use_the_outer_block() {
        let diags = crate::testutil::run_cop_full_with_config(
            &ArgumentsForwarding,
            b"def foo(&block)\n  def inner\n    block\n  end\n  bar(&block)\nend\n",
            ruby34(),
        );
        assert_eq!(diags.len(), 2, "{diags:?}");
    }
}
