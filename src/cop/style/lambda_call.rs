//! Style/LambdaCall — lambda.call vs lambda.().

use tree_sitter::Node;

use crate::cop::shared::call_method_name;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LambdaCall;

impl Cop for LambdaCall {
    fn name(&self) -> &'static str {
        "Style/LambdaCall"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"call"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "call");
        let Some(method) = call_method_name(source, node) else {
            return;
        };
        if style == "call" && method == b"call" {
            return;
        }
        if style == "braces" && method == b"call" {
            report_braces(self, source, node, diagnostics, &mut corrections);
        }
    }
}

fn report_braces(
    cop: &LambdaCall,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let meth = node.child_by_field_name("method").unwrap_or(node);
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Prefer the use of `lambda.()` to call a lambda.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: meth.start_byte(),
            end: meth.end_byte(),
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
