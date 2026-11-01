use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::model::{IntroKind, ScopeKind};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

/// Lint/UnusedBlockArgument — unused block params.
pub struct UnusedBlockArgument;

fn block_at_offset<'a>(root: Node<'a>, offset: usize) -> Option<Node<'a>> {
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if matches!(n.kind(), "block" | "do_block") && n.start_byte() == offset {
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

fn block_body_empty(tree: &tree_sitter::Tree, entered_at: usize) -> bool {
    let Some(block) = block_at_offset(tree.root_node(), entered_at) else {
        return false;
    };
    let body = block.child_by_field_name("body").or_else(|| {
        let mut cur = block.walk();
        block
            .named_children(&mut cur)
            .find(|c| matches!(c.kind(), "block_body" | "body_statement"))
    });
    let Some(body) = body else {
        return true;
    };
    let mut cur = body.walk();
    !body
        .named_children(&mut cur)
        .any(|c| !matches!(c.kind(), "comment"))
}

impl Cop for UnusedBlockArgument {
    fn name(&self) -> &'static str {
        "Lint/UnusedBlockArgument"
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
        file_model: &crate::model::FileModel<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let ignore_empty = config.get_bool("IgnoreEmptyBlocks", true);
        for scope in file_model.scopes.iter() {
            if scope.kind != ScopeKind::Block {
                continue;
            }
            if ignore_empty && block_body_empty(&file_model.tree, scope.entered_at) {
                continue;
            }
            for (name, entry) in &scope.entries {
                if entry.intro_kind != IntroKind::Binding {
                    continue;
                }
                if name.starts_with('_') {
                    continue;
                }
                if !entry.reads.is_empty() {
                    continue;
                }
                let (line, col) = file_model.line_col(entry.intro_byte);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    col,
                    format!(
                        "Unused block argument - `{name}`. If it's necessary, use `_{name}` as an argument name to indicate that it won't be used."
                    ),
                ));
            }
        }
    }
}
