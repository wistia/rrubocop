//! Rails/ApplicationJob — breadth-first tree-sitter port.

use tree_sitter::Node;

use super::enforce_superclass::check_superclass;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ApplicationJob;

impl Cop for ApplicationJob {
    fn name(&self) -> &'static str {
        "Rails/ApplicationJob"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn safe_autocorrect(&self) -> bool {
        false
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["class", "call"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        check_superclass(
            self,
            source,
            node,
            "ActiveJob::Base",
            "ApplicationJob",
            "Jobs should subclass `ApplicationJob`.",
            config,
            diagnostics,
            corrections,
        );
    }
}
