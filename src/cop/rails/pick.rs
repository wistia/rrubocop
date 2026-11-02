//! Fetch one projected value instead of materializing an entire pluck result.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, method_node, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct Pick;
impl Cop for Pick {
    fn name(&self) -> &'static str {
        "Rails/Pick"
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
        &[b"first"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_f64("TargetRailsVersion", 8.1) < 6.0
            || call_method_name(source, node) != Some(b"first")
            || !argument_nodes(node).is_empty()
        {
            return;
        }
        let Some(query) = call_receiver(node) else {
            return;
        };
        if call_method_name(source, query) != Some(b"pluck")
            || query.child_by_field_name("block").is_some()
        {
            return;
        }
        let Some(selector) = method_node(query) else {
            return;
        };
        let Some(last) = method_node(node) else {
            return;
        };
        let args = argument_nodes(query)
            .iter()
            .map(|n| node_text(source, *n))
            .collect::<Vec<_>>()
            .join(", ");
        let current =
            String::from_utf8_lossy(&source.as_bytes()[selector.start_byte()..last.end_byte()]);
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Prefer `pick({args})` over `{current}`."),
        );
        d.highlight_length = current.lines().next().unwrap_or("").chars().count();
        d.correctable = !config.autocorrect_disabled()
            && !source.as_bytes()[query.end_byte()..last.end_byte()].contains(&b'#');
        if node.child_by_field_name("arguments").is_some() {
            d.correctable = false;
        }
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                selector.end_byte(),
                "pick",
                self.name(),
            );
            push_replace(
                &mut corrections,
                query.end_byte(),
                last.end_byte(),
                "",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Pick, "cops/rails/pick");

    #[test]
    fn respects_the_minimum_rails_version() {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRailsVersion".into(),
            serde_yml::to_value(5.2).unwrap(),
        );
        assert!(
            crate::testutil::run_cop_full_with_config(&Pick, b"User.pluck(:name).first\n", config)
                .is_empty()
        );
    }

    #[test]
    fn does_not_produce_invalid_ruby_from_empty_outer_parentheses() {
        let before = b"User.pluck(:name).first()\n";
        let source = SourceFile::from_bytes("test.rb", before.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let mut diagnostics = vec![];
        let mut edits = vec![];
        crate::cop::walker::BatchedWalker::new(vec![&Pick], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut edits),
        );
        assert!(edits.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }
}
