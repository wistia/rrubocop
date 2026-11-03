//! GraphQL/MultipleFieldDefinitions — group duplicate-named field definitions.

use tree_sitter::Node;

use super::helpers::{
    CALL_KINDS, DEPT_INCLUDE, class_body_stmts, enclosing_class, field_name, is_field_call,
    nested_class,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultipleFieldDefinitions;

impl Cop for MultipleFieldDefinitions {
    fn name(&self) -> &'static str {
        "GraphQL/MultipleFieldDefinitions"
    }

    fn default_include(&self) -> &'static [&'static str] {
        DEPT_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        CALL_KINDS
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        super::helpers::FIELD_METHODS
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !is_field_call(source, node) {
            return;
        }
        let Some(class) = enclosing_class(node) else {
            return;
        };
        if nested_class(class) {
            return;
        }
        if !ungrouped_last_def(source, class, node) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "Group multiple field definitions together.".into(),
        ));
    }
}

fn ungrouped_last_def(source: &SourceFile, class: Node<'_>, node: Node<'_>) -> bool {
    struct Ungrouped(std::collections::HashSet<usize>);
    let index = source.cached(class.start_byte(), || {
        let mut definitions = std::collections::HashMap::<String, Vec<(usize, usize)>>::new();
        for (position, statement) in class_body_stmts(class).into_iter().enumerate() {
            if is_field_call(source, statement) {
                if let Some(name) = field_name(source, statement) {
                    definitions
                        .entry(name)
                        .or_default()
                        .push((position, statement.start_byte()));
                }
            }
        }
        Ungrouped(
            definitions
                .into_values()
                .filter_map(|definitions| {
                    (definitions.len() > 1
                        && definitions.windows(2).any(|pair| pair[1].0 - pair[0].0 > 1))
                    .then(|| definitions.last().unwrap().1)
                })
                .collect(),
        )
    });
    index.0.contains(&node.start_byte())
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    #[test]
    fn class_index_retains_statement_gaps_and_class_boundaries() {
        let code = b"class A
 field :x, String
 field :y, String
 field :x, String
end
class B
 field :x, String
 field :x, String
end
";
        let offenses = crate::testutil::run_cop_full(&MultipleFieldDefinitions, code);
        assert_eq!(offenses.len(), 1);
        assert_eq!(offenses[0].location.line, 4);
    }
}
