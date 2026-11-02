//! Remove callback options ignored by Rails option precedence.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, named_kids, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct IgnoredSkipActionFilterOption;
fn side_effect_free(source: &SourceFile, node: Node<'_>) -> bool {
    if crate::cop::ported::static_literal(source, node).is_some() {
        return true;
    }
    match node.kind() {
        "integer" | "float" | "true" | "false" | "nil" | "lambda" => true,
        "identifier" => crate::cop::ported::local_identifier(source, node),
        "array" | "hash" | "pair" => named_kids(node)
            .into_iter()
            .all(|n| side_effect_free(source, n)),
        _ => false,
    }
}
impl Cop for IgnoredSkipActionFilterOption {
    fn name(&self) -> &'static str {
        "Rails/IgnoredSkipActionFilterOption"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/controllers/**/*.rb", "**/app/mailers/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[
            b"skip_before_action",
            b"skip_after_action",
            b"skip_around_action",
            b"skip_action_callback",
        ]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !matches!(
            call_method_name(source, node),
            Some(
                b"skip_before_action"
                    | b"skip_after_action"
                    | b"skip_around_action"
                    | b"skip_action_callback"
            )
        ) || call_receiver(node).is_some()
        {
            return;
        }
        let args = argument_nodes(node);
        let Some(last) = args.last() else {
            return;
        };
        let explicit = last.kind() == "hash";
        let pairs = if explicit {
            named_kids(*last)
        } else {
            let start = args
                .iter()
                .rposition(|n| !matches!(n.kind(), "pair" | "hash_splat_argument"))
                .map_or(0, |i| i + 1);
            args[start..].to_vec()
        };
        let mut conditional = None;
        let mut only = None;
        let mut except = None;
        for pair in &pairs {
            let Some(key) = pair.child_by_field_name("key") else {
                continue;
            };
            match crate::cop::ported::static_symbol(source, key).as_deref() {
                Some("if") => conditional = Some(*pair),
                Some("only") => only = Some(*pair),
                Some("except") => except = Some(*pair),
                _ => {}
            }
        }
        let (target, prefer, ignore) = match (conditional, only, except) {
            (Some(target), Some(_), _) => (target, "only", "if"),
            (Some(_), None, Some(target)) => (target, "if", "except"),
            _ => return,
        };
        let (line, col) = source.offset_to_line_col(target.start_byte());
        let mut d=self.diagnostic(source,line,col,format!("`{ignore}` option will be ignored when `{prefer}` and `{ignore}` are used together."));
        d.highlight_length =
            super::blank::highlight_length(source, target.start_byte(), target.end_byte());
        let mut start = target.start_byte();
        let bytes = source.as_bytes();
        while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        while start > 0 && bytes[start - 1] == b'\n' {
            start -= 1;
        }
        while start > 0 && bytes[start - 1] == b',' {
            start -= 1;
        }
        let value = target.child_by_field_name("value");
        d.correctable = !config.autocorrect_disabled()
            && value.is_some_and(|n| side_effect_free(source, n))
            && !(explicit && pairs.first() == Some(&target))
            && !collect_comments(target)
                .iter()
                .any(|c| c.start_byte() >= start);
        if d.correctable {
            d.corrected = push_replace(&mut corrections, start, target.end_byte(), "", self.name());
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IgnoredSkipActionFilterOption,
        "cops/rails/ignored_skip_action_filter_option"
    );
}
