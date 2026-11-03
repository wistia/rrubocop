//! Report the original exit selector without treating local variables as calls.
use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::ruby_expression::{regular_send_name, regular_send_receiver};
use crate::cop::shared::{call_method_name, is_const_named, method_node};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct Exit;
fn allowed_receiver(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(receiver) = regular_send_receiver(source, node) else {
        return false;
    };
    receiver.is_none_or(|receiver| {
        is_const_named(source, receiver, b"Kernel")
            || is_const_named(source, receiver, b"Process")
            || (receiver.kind() == "call"
                && receiver.child_by_field_name("block").is_none()
                && call_method_name(source, receiver)
                    .is_some_and(|name| matches!(name, b"Kernel" | b"Process")))
    })
}
impl Cop for Exit {
    fn name(&self) -> &'static str {
        "Rails/Exit"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/**/*.rb", "**/config/**/*.rb", "**/lib/**/*.rb"]
    }
    fn default_exclude(&self) -> &'static [&'static str] {
        &["**/lib/**/*.rake"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !regular_send_name(source, node)
            .is_some_and(|name| matches!(name.as_ref(), b"exit" | b"exit!"))
            || !allowed_receiver(source, node)
            || arguments(node).len() > 1
        {
            return;
        }
        let selector = method_node(node).unwrap_or(node);
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Do not use `exit` in Rails applications.".into(),
        );
        d.highlight_length = selector.end_byte() - selector.start_byte();
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Exit, "cops/rails/exit");
}
