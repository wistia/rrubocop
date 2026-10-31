use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, call_receiver};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/DeprecatedClassMethods — iterator?, attr with bool, etc.
pub struct DeprecatedClassMethods;

fn replace_method(
    cop: &DeprecatedClassMethods,
    source: &SourceFile,
    node: Node<'_>,
    msg: String,
    replacement: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let meth = node.child_by_field_name("method").unwrap_or(node);
    let (line, col) = source.offset_to_line_col(meth.start_byte());
    let mut diag = cop.diagnostic(source, line, col, msg);
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: meth.start_byte(),
            end: meth.end_byte(),
            replacement: replacement.to_string(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn check_iterator(
    cop: &DeprecatedClassMethods,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) -> bool {
    if call_method_name(source, node) != Some(b"iterator?") {
        return false;
    }
    replace_method(
        cop,
        source,
        node,
        "`iterator?` is deprecated in favor of `block_given?`.".to_string(),
        "block_given?",
        diagnostics,
        corrections,
    );
    true
}

fn attr_prefer(args: &[Node<'_>]) -> Option<&'static str> {
    if args.len() != 2 || !matches!(args[1].kind(), "true" | "false") {
        return None;
    }
    Some(if args[1].kind() == "true" {
        "attr_accessor"
    } else {
        "attr_reader"
    })
}

fn drop_bool_arg(node: Node<'_>, args: &[Node<'_>], cop_name: &'static str) -> Correction {
    let mut cur = node.walk();
    Correction {
        start: args[0].end_byte(),
        end: node
            .children(&mut cur)
            .find(|c| !c.is_named() && c.kind() == ")")
            .map(|c| c.start_byte())
            .unwrap_or(node.end_byte()),
        replacement: String::new(),
        cop_name,
        cop_index: 0,
        owner: None,
    }
}

fn check_attr(
    cop: &DeprecatedClassMethods,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if call_method_name(source, node) != Some(b"attr") || call_receiver(node).is_some() {
        return;
    }
    let args = argument_nodes(node);
    let Some(prefer) = attr_prefer(&args) else {
        return;
    };
    replace_method(
        cop,
        source,
        node,
        format!("`attr` is deprecated in favor of `{prefer}`."),
        prefer,
        diagnostics,
        corrections,
    );
    if let Some(corr) = corrections.as_mut() {
        // replace_method already marked corrected; also drop bool arg
        corr.push(drop_bool_arg(node, &args, cop.name()));
    }
}

impl Cop for DeprecatedClassMethods {
    fn name(&self) -> &'static str {
        "Lint/DeprecatedClassMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if check_iterator(self, source, node, diagnostics, &mut corrections) {
            return;
        }
        check_attr(self, source, node, diagnostics, &mut corrections);
    }
}
