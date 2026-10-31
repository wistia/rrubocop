//! Redundant `&block` / `&blk` / `&proc` forwarded as anonymous `&`.

use tree_sitter::Node;

use crate::cop::shared::{
    EXCLUDED_BLOCK_PARENTS, METHOD_STOPS, NESTED_SCOPES, ScanOpts, replace_arg, scan,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::{ArgumentsForwarding, COP, method_body, param_name};

const BLOCK_MSG: &str = "Use anonymous block arguments forwarding (`&`).";

pub(super) fn report(
    cop: &ArgumentsForwarding,
    source: &SourceFile,
    method: Node<'_>,
    params: Node<'_>,
    config: &CopConfig,
    ruby: f64,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some((name, param, body)) = candidate(source, method, params, config, ruby) else {
        return;
    };
    let found = scan(
        source,
        body,
        name,
        Some(name),
        &ScanOpts {
            skip_scopes: NESTED_SCOPES,
            excluded_parents: EXCLUDED_BLOCK_PARENTS,
            scope_stops: METHOD_STOPS,
            include_lambda: true,
            any_inside_block: false,
            guard_inside_block: ruby < 3.4 || super::legacy_block_forwarding(config),
        },
    );
    if found.name_used || found.blocked || found.nodes.is_empty() {
        return;
    }
    if super::legacy_block_forwarding(config) && found.nodes.iter().any(|n| super::within_block(*n))
    {
        return;
    }
    for pass in found.nodes {
        report_block(cop, source, pass, diagnostics, corrections);
    }
    report_block(cop, source, param, diagnostics, corrections);
}

fn candidate<'a>(
    source: &'a SourceFile,
    method: Node<'a>,
    params: Node<'a>,
    config: &CopConfig,
    ruby: f64,
) -> Option<(&'a [u8], Node<'a>, Node<'a>)> {
    if ruby < 3.1 || (ruby >= 3.2 && !config.get_bool("UseAnonymousForwarding", true)) {
        return None;
    }
    let param = block_param(params)?;
    let name = param_name(source, param)?;
    if !redundant_name(config, name) {
        return None;
    }
    let body = method_body(method)?;
    Some((name, param, body))
}

fn redundant_name(config: &CopConfig, name: &[u8]) -> bool {
    let Ok(name) = std::str::from_utf8(name) else {
        return false;
    };
    match config.options.get("RedundantBlockArgumentNames") {
        Some(serde_yml::Value::Sequence(items)) => items.iter().any(|v| v.as_str() == Some(name)),
        _ => matches!(name, "blk" | "block" | "proc"),
    }
}

fn block_param(params: Node<'_>) -> Option<Node<'_>> {
    let mut cur = params.walk();
    params
        .named_children(&mut cur)
        .find(|n| n.kind() == "block_parameter")
}

fn report_block(
    cop: &ArgumentsForwarding,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(source, line, col, BLOCK_MSG.to_string());
    diag.highlight_length = node.end_byte().saturating_sub(node.start_byte()).max(1);
    if replace_arg(source, node, "&", corrections, COP) {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
