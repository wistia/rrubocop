//! Redundant `**kwargs` / `**options` / `**opts` forwarded as anonymous `**`.

mod body;

use tree_sitter::Node;

use crate::cop::shared::replace_arg;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use body::{in_block, scan};

use super::{ArgumentsForwarding, COP, method_body, param_name};

const KW_MSG: &str = "Use anonymous keyword arguments forwarding (`**`).";

pub(super) fn report(
    cop: &ArgumentsForwarding,
    source: &SourceFile,
    method: Node<'_>,
    params: Node<'_>,
    config: &CopConfig,
    ruby: f64,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some((param, forwards)) = candidate(source, method, params, config, ruby) else {
        return;
    };
    // Before Ruby 3.4, anonymous forwarding inside a block is left unchanged.
    if (!at_least(ruby, 3.4) || super::legacy_block_forwarding(config))
        && forwards.iter().any(|node| {
            if super::legacy_block_forwarding(config) {
                super::within_block(*node)
            } else {
                in_block(*node)
            }
        })
    {
        return;
    }
    for node in &forwards {
        emit(cop, source, *node, diagnostics, corrections);
    }
    emit(cop, source, param, diagnostics, corrections);
}

fn candidate<'a>(
    source: &'a SourceFile,
    method: Node<'a>,
    params: Node<'a>,
    config: &CopConfig,
    ruby: f64,
) -> Option<(Node<'a>, Vec<Node<'a>>)> {
    if !(at_least(ruby, 3.2) && config.get_bool("UseAnonymousForwarding", true)) {
        return None;
    }
    let param = kw_param(params)?;
    let name = param_name(source, param)?;
    if !redundant(config, name) {
        return None;
    }
    let found = scan(source, method_body(method)?, name);
    (!found.used && !found.forwards.is_empty()).then_some((param, found.forwards))
}

fn at_least(ruby: f64, version: f64) -> bool {
    (ruby * 10.0).round() >= (version * 10.0).round()
}

fn redundant(config: &CopConfig, name: &[u8]) -> bool {
    let Ok(name) = std::str::from_utf8(name) else {
        return false;
    };
    match config.options.get("RedundantKeywordRestArgumentNames") {
        Some(serde_yml::Value::Sequence(items)) => items.iter().any(|v| v.as_str() == Some(name)),
        _ => matches!(name, "kwargs" | "options" | "opts"),
    }
}

fn kw_param(params: Node<'_>) -> Option<Node<'_>> {
    let mut cur = params.walk();
    params
        .named_children(&mut cur)
        .find(|n| n.kind() == "hash_splat_parameter")
}

fn emit(
    cop: &ArgumentsForwarding,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(source, line, col, KW_MSG.to_string());
    diag.highlight_length = node.end_byte().saturating_sub(node.start_byte()).max(1);
    if replace_arg(source, node, "**", corrections, COP) {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFENSE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cops/style/arguments_forwarding/kw_offense.rb"
    ));
    const CLEAN: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cops/style/arguments_forwarding/kw_no_offense.rb"
    ));

    fn ruby(ver: f64) -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".into(),
            serde_yml::Value::Number(serde_yml::Number::from(ver)),
        );
        config
    }

    fn ruby_flag(ver: f64, forwarding: bool) -> CopConfig {
        let mut config = ruby(ver);
        config.options.insert(
            "UseAnonymousForwarding".into(),
            serde_yml::Value::Bool(forwarding),
        );
        config
    }

    #[test]
    fn keyword_rest_ruby34_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &ArgumentsForwarding,
            OFFENSE,
            ruby(3.4),
        );
    }

    #[test]
    fn keyword_rest_ruby34_clean_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &ArgumentsForwarding,
            CLEAN,
            ruby(3.4),
        );
    }

    fn walk_src(src: &str, ver: f64) -> (SourceFile, Vec<Correction>) {
        let source = SourceFile::from_bytes("t.rb", src.as_bytes().to_vec());
        let tree = crate::parse::parse_ruby(&source).expect("parse");
        let mut corrections = Vec::new();
        crate::cop::walker::BatchedWalker::new(vec![&ArgumentsForwarding], vec![&ruby(ver)]).walk(
            &source,
            tree.root_node(),
            &mut Vec::new(),
            Some(&mut corrections),
        );
        (source, corrections)
    }

    fn corrected(src: &str, ver: f64) -> String {
        let (source, corrections) = walk_src(src, ver);
        String::from_utf8(
            crate::correction::CorrectionSet::from_vec(corrections).apply(source.as_bytes()),
        )
        .expect("utf8")
    }

    #[test]
    fn register_autocorrect() {
        let src = "\
def register(method_name, instrument, **options)
  published = \"#{name.demodulize.underscore}.#{method_name}\"
  var = :\"@#{method_name}\"
  define_singleton_method(method_name) do
    instance_variable_get(var) || instance_variable_set(var, instrument.build(published, **options))
  end
end
";
        let expect = "\
def register(method_name, instrument, **)
  published = \"#{name.demodulize.underscore}.#{method_name}\"
  var = :\"@#{method_name}\"
  define_singleton_method(method_name) do
    instance_variable_get(var) || instance_variable_set(var, instrument.build(published, **))
  end
end
";
        assert_eq!(corrected(src, 3.4), expect);
    }

    #[test]
    fn bare_kwargs_gain_parentheses() {
        assert_eq!(
            corrected("def foo **options\n  bar **options\nend\n", 3.2),
            "def foo(**)\n  bar(**)\nend\n"
        );
    }

    #[test]
    fn block_holds_kwargs_before_ruby34() {
        let src = b"def foo(**options)\n  bar do\n    baz(**options)\n  end\nend\n";
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &ArgumentsForwarding,
            src,
            ruby(3.3),
        );
        let diags = crate::testutil::run_cop_full_with_config(&ArgumentsForwarding, src, ruby(3.4));
        assert_eq!(diags.len(), 2, "{diags:?}");
    }

    #[test]
    fn forwarding_disabled() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &ArgumentsForwarding,
            b"def foo(**options)\n  bar(**options)\nend\n",
            ruby_flag(3.4, false),
        );
    }
}
