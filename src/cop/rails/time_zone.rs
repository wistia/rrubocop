//! Rails/TimeZone — Time methods without zone (tree-sitter port).

mod detect;
pub(super) use detect::ACCEPTED as ACCEPTED_METHODS;

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use detect::{
    DANGEROUS, attach_tz_string, build_message, chain_from, has_in_kwarg, method_name,
    not_danger_chain, offset_provided, selector_off, string_to_time_needs_zone, time_receiver,
};

pub struct TimeZone;

fn report_time_zone(
    cop: &TimeZone,
    source: &SourceFile,
    node: Node<'_>,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, col) = source.offset_to_line_col(selector_off(node));
    diagnostics.push(cop.diagnostic(source, line, col, msg));
}

fn check_time_call(
    cop: &TimeZone,
    source: &SourceFile,
    node: Node<'_>,
    flexible: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if time_receiver(source, node).is_none() {
        return;
    }
    let Some(method) = method_name(source, node) else {
        return;
    };
    if !DANGEROUS.iter().any(|&d| d == method) {
        return;
    }
    if attach_tz_string(source, node) || has_in_kwarg(source, node) || offset_provided(node, method)
    {
        return;
    }
    if not_danger_chain(&chain_from(source, node), flexible) {
        return;
    }
    report_time_zone(
        cop,
        source,
        node,
        build_message(flexible, &String::from_utf8_lossy(method)),
        diagnostics,
    );
}

fn legacy_timezone_literal(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    if config
        .get_str("__RubocopRailsVersion", "2.37.0")
        .starts_with("2.26.")
    {
        if crate::cop::shared::argument_nodes(node)
            .first()
            .is_some_and(|n| {
                n.kind() == "string"
                    && crate::cop::shared::node_bytes(source, *n)
                        .iter()
                        .rev()
                        .skip(1)
                        .next()
                        .is_some_and(|b| b.is_ascii_alphabetic())
            })
        {
            return true;
        }
    }
    false
}

impl Cop for TimeZone {
    fn name(&self) -> &'static str {
        "Rails/TimeZone"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn safe_autocorrect(&self) -> bool {
        false
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if legacy_timezone_literal(source, node, config) {
            return;
        }
        let flexible = config.get_str("EnforcedStyle", "flexible") != "strict";
        if string_to_time_needs_zone(source, node) {
            report_time_zone(
                self,
                source,
                node,
                "Do not use `String#to_time` without zone. Use `Time.zone.parse` instead.".into(),
                diagnostics,
            );
            return;
        }
        check_time_call(self, source, node, flexible, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TimeZone, "cops/rails/time_zone");
}
