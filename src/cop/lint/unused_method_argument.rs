//! Lint/UnusedMethodArgument — unused method parameters.

use tree_sitter::Node;

use crate::cop::shared::{for_each_descendant, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::model::{FileModel, IntroKind, ScopeKind};
use crate::parse::source::SourceFile;

pub struct UnusedMethodArgument;

fn method_at_offset<'a>(root: Node<'a>, offset: usize) -> Option<Node<'a>> {
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if matches!(n.kind(), "method" | "singleton_method") && n.start_byte() == offset {
            return Some(n);
        }
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i as u32)
                && c.end_byte() > offset
                && c.start_byte() <= offset
            {
                stack.push(c);
            }
        }
    }
    None
}

fn method_body_empty(tree: &tree_sitter::Tree, entered_at: usize) -> bool {
    method_at_offset(tree.root_node(), entered_at)
        .is_some_and(|m| m.child_by_field_name("body").is_none())
}

fn single_raise_or_fail(source: &SourceFile, body: Node<'_>) -> bool {
    let mut cur = body.walk();
    let stmts: Vec<_> = body
        .named_children(&mut cur)
        .filter(|c| c.kind() != "comment")
        .collect();
    let Some(n) = stmts.first().filter(|_| stmts.len() == 1) else {
        return false;
    };
    if n.kind() != "call" {
        return false;
    }
    n.child_by_field_name("method")
        .is_some_and(|m| matches!(node_text(source, m).as_str(), "raise" | "fail"))
}

fn body_not_implemented(source: &SourceFile, tree: &tree_sitter::Tree, entered_at: usize) -> bool {
    method_at_offset(tree.root_node(), entered_at)
        .and_then(|m| m.child_by_field_name("body"))
        .is_some_and(|b| single_raise_or_fail(source, b))
}

fn is_keyword_param(tree: &tree_sitter::Tree, intro_byte: usize) -> bool {
    let mut found = false;
    for_each_descendant(tree.root_node(), |n| {
        if n.kind() == "keyword_parameter"
            && n.child_by_field_name("name")
                .is_some_and(|nm| nm.start_byte() == intro_byte)
        {
            found = true;
        }
    });
    found
}

fn skip_method(
    source: &SourceFile,
    fm: &FileModel<'_>,
    entered_at: usize,
    config: &CopConfig,
) -> bool {
    let empty =
        config.get_bool("IgnoreEmptyMethods", true) && method_body_empty(&fm.tree, entered_at);
    let nyi = config.get_bool("IgnoreNotImplementedMethods", true)
        && body_not_implemented(source, &fm.tree, entered_at);
    empty || nyi
}

fn unused_msg(name: &str, method: Option<&str>, all_unused: bool) -> String {
    let mut msg = format!(
        "Unused method argument - `{name}`. If it's necessary, use `_` or `_{name}` as an argument name to indicate that it won't be used. If it's unnecessary, remove it."
    );
    if all_unused {
        if let Some(m) = method {
            msg.push_str(&format!(
                " You can also write as `{m}(*)` if you want the method to accept any arguments but don't care about them."
            ));
        }
    }
    msg
}

fn method_name(source: &SourceFile, tree: &tree_sitter::Tree, entered_at: usize) -> Option<String> {
    method_at_offset(tree.root_node(), entered_at)
        .and_then(|m| m.child_by_field_name("name"))
        .map(|n| node_text(source, n))
}

fn all_params_unused(scope: &crate::model::ScopeData) -> bool {
    scope
        .entries
        .values()
        .filter(|e| e.intro_kind == IntroKind::Param)
        .all(|e| e.reads.is_empty())
}

fn skip_entry(
    name: &str,
    entry: &crate::model::Entry,
    allow_kw: bool,
    tree: &tree_sitter::Tree,
) -> bool {
    name.starts_with('_')
        || entry.intro_kind != IntroKind::Param
        || !entry.reads.is_empty()
        || (allow_kw && is_keyword_param(tree, entry.intro_byte))
}

fn report_scope(
    cop: &UnusedMethodArgument,
    source: &SourceFile,
    fm: &FileModel<'_>,
    scope: &crate::model::ScopeData,
    allow_kw: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let all_unused = all_params_unused(scope);
    let meth = method_name(source, &fm.tree, scope.entered_at);
    for (name, entry) in &scope.entries {
        if skip_entry(name, entry, allow_kw, &fm.tree) {
            continue;
        }
        let (line, col) = fm.line_col(entry.intro_byte);
        diagnostics.push(cop.diagnostic(
            source,
            line,
            col,
            unused_msg(name, meth.as_deref(), all_unused),
        ));
    }
}

impl Cop for UnusedMethodArgument {
    fn name(&self) -> &'static str {
        "Lint/UnusedMethodArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn needs_file_model(&self) -> bool {
        true
    }

    fn check_file_model(
        &self,
        source: &SourceFile,
        file_model: &FileModel<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_kw = config.get_bool("AllowUnusedKeywordArguments", false);
        for scope in file_model.scopes.iter() {
            if scope.kind != ScopeKind::Method
                || skip_method(source, file_model, scope.entered_at, config)
            {
                continue;
            }
            report_scope(self, source, file_model, scope, allow_kw, diagnostics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnusedMethodArgument, "cops/lint/unused_method_argument");
}
