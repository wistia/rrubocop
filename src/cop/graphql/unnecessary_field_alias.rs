//! GraphQL/UnnecessaryFieldAlias — alias/method/etc. matching the field name.

use tree_sitter::Node;

use super::helpers::{CALL_KINDS, DEPT_INCLUDE, field_name, is_field_call, kwarg_sym_value};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

const ALIAS_KEYS: &[&str] = &["alias", "method", "resolver_method", "hash_key"];

pub struct UnnecessaryFieldAlias;

impl Cop for UnnecessaryFieldAlias {
    fn name(&self) -> &'static str {
        "GraphQL/UnnecessaryFieldAlias"
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
        let Some(name) = field_name(source, node) else {
            return;
        };
        let Some(kw) = matching_alias_key(source, node, &name) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            format!("Unnecessary :{kw} configured"),
        ));
    }
}

fn matching_alias_key<'a>(source: &SourceFile, node: Node<'_>, name: &str) -> Option<&'a str> {
    ALIAS_KEYS
        .iter()
        .copied()
        .find(|k| kwarg_sym_value(source, node, k).as_deref() == Some(name))
}
