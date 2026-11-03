use crate::cop::shared::ruby_arguments::{Argument, arguments};
use crate::cop::shared::ruby_expression::{
    numbered_bindings, plain_string, plain_symbol, regular_send_name, regular_send_receiver,
};
use crate::cop::shared::{for_each_descendant, named_kids, push_replace, send_end};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct RakeEnvironment;
fn literal_name(source: &SourceFile, node: Node<'_>) -> Option<String> {
    plain_symbol(source, node).or_else(|| plain_string(source, node))
}
fn array(node: Node<'_>) -> bool {
    matches!(node.kind(), "array" | "string_array" | "symbol_array")
}
fn task_name(source: &SourceFile, first: &Argument<'_>) -> Option<String> {
    if !first.hash {
        return literal_name(source, first.node()?);
    }
    let parts = first.parts();
    if parts.len() != 1 || parts[0].kind() != "pair" {
        return None;
    }
    literal_name(source, parts[0].child_by_field_name("key")?)
}
fn dependencies(hash: &Argument<'_>) -> bool {
    let Some(value) = hash
        .parts()
        .into_iter()
        .find(|n| n.kind() == "pair")
        .and_then(|n| n.child_by_field_name("value"))
    else {
        return false;
    };
    !array(value) || named_kids(value).iter().any(|n| n.kind() != "comment")
}
fn valid_label(value: &str) -> bool {
    let source = SourceFile::from_bytes(
        "rake-label.rb",
        format!("{{{value}: :environment}}\n").into_bytes(),
    );
    let Ok(tree) = crate::parse::parse_ruby(&source) else {
        return false;
    };
    if tree.root_node().has_error() {
        return false;
    }
    let mut found = false;
    for_each_descendant(tree.root_node(), |n| {
        if n.kind() == "pair"
            && n.child_by_field_name("key")
                .and_then(|n| plain_symbol(&source, n))
                .as_deref()
                == Some(value)
        {
            found = true;
        }
    });
    found
}
impl Cop for RakeEnvironment {
    fn name(&self) -> &'static str {
        "Rails/RakeEnvironment"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["**/Rakefile", "**/*.rake"]
    }
    fn default_exclude(&self) -> &'static [&'static str] {
        &["lib/capistrano/tasks/**/*.rake"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"task"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if regular_send_receiver(source, node) != Some(None)
            || !regular_send_name(source, node).is_some_and(|n| n.as_ref() == b"task")
        {
            return;
        }
        let Some(block) = node.child_by_field_name("block") else {
            return;
        };
        if config.get_f64("TargetRubyVersion", 3.4) >= 2.7
            && numbered_bindings(source, block).is_some()
        {
            return;
        }
        let args = arguments(node);
        let first = args.first();
        if first.and_then(|a| task_name(source, a)).as_deref() == Some("default") {
            return;
        }
        let hash = first
            .filter(|a| a.hash)
            .or_else(|| args.get(1).filter(|a| a.hash));
        if hash.is_some_and(dependencies) {
            return;
        }
        let target = args
            .get(1)
            .filter(|a| a.node().is_some_and(array))
            .or(first);
        let mut replacement = None;
        if let Some(target) = target {
            let raw = String::from_utf8_lossy(&source.as_bytes()[target.start()..target.end()]);
            if args
                .get(1)
                .is_some_and(|a| a.start() == target.start() && a.node().is_some_and(array))
            {
                replacement = Some(format!("{raw} => :environment"));
            } else if let Some(symbol) = target.node().and_then(|n| plain_symbol(source, n)) {
                let name: String = raw
                    .chars()
                    .filter(|c| !matches!(c, ':' | '|' | '\'' | '"'))
                    .collect();
                if name == symbol && valid_label(&name) {
                    replacement = Some(format!("{name}: :environment"));
                }
            } else {
                replacement = Some(format!("{raw} => :environment"));
            }
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Include `:environment` task as a dependency for all Rake tasks.".into(),
        );
        d.highlight_length =
            super::blank::highlight_length(source, node.start_byte(), send_end(node));
        if let Some(s) = config.severity {
            d.severity = s;
        }
        d.correctable = !config.autocorrect_disabled()
            && replacement.is_some()
            && !target.is_some_and(|t| t.hash && !t.braces);
        if d.correctable {
            let target = target.unwrap();
            d.corrected = push_replace(
                &mut corrections,
                target.start(),
                target.end(),
                replacement.unwrap(),
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RakeEnvironment, "cops/rails/rake_environment");
}
