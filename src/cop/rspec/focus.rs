use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{call_method_name, call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct Focus;
impl Cop for Focus {
    fn name(&self) -> &'static str {
        "RSpec/Focus"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        crate::cop::rspec::helpers::example_and_group_names()
    }
    fn interested_identifier_names(&self) -> &'static [&'static [u8]] {
        self.interested_call_names()
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let method_name = if node.kind() == "identifier" {
            Some(node_bytes(source, node))
        } else {
            call_method_name(source, node)
        };
        if !method_name.is_some_and(|method| {
            crate::cop::rspec::helpers::is_group(method)
                || crate::cop::rspec::helpers::is_example(method)
        }) {
            return;
        }
        let _ = config;
        if safe_send(source, node)
            || node
                .parent()
                .is_some_and(|parent| call_receiver(parent) == Some(node))
        {
            return;
        }
        let mut ancestor = node.parent();
        while let Some(parent) = ancestor {
            if matches!(parent.kind(), "method" | "singleton_method") {
                return;
            }
            ancestor = parent.parent();
        }
        let Some(method) = bare_rspec_call(source, node).or_else(|| {
            (node.kind() == "identifier")
                .then(|| name(source, node))
                .flatten()
        }) else {
            return;
        };
        let focused = [
            b"fdescribe".as_slice(),
            b"fcontext",
            b"ffeature",
            b"fit",
            b"fexample",
            b"fspecify",
            b"fscenario",
            b"focus",
        ]
        .contains(&method);
        if focused {
            let index = report(
                self,
                source,
                node.start_byte(),
                header_end(node),
                "Focused spec found.".into(),
                diagnostics,
            );
            diagnostics[index].correctable = method != b"focus";
            if method != b"focus" {
                let target = selector(node);
                let replacement = String::from_utf8_lossy(&method[1..]);
                if preserving_replace(
                    source,
                    node,
                    &mut corrections,
                    target.start_byte(),
                    target.end_byte(),
                    replacement,
                    self.name(),
                ) {
                    diagnostics[index].corrected = true;
                }
            }
            return;
        }
        if !(crate::cop::rspec::helpers::is_group(method)
            || crate::cop::rspec::helpers::is_example(method))
        {
            return;
        }
        let mut targets = Vec::new();
        for arg in call_args(node) {
            if symbol_node(arg)
                && crate::cop::ported::static_literal(source, arg).as_deref() == Some("focus")
            {
                targets.push(arg);
            }
            let pairs = if arg.kind() == "hash" {
                code_children(arg)
            } else if arg.kind() == "pair" {
                vec![arg]
            } else {
                vec![]
            };
            for pair in pairs {
                if pair.child_by_field_name("key").is_some_and(|key| {
                    (symbol_node(key) || key.kind() == "hash_key_symbol")
                        && crate::cop::ported::static_literal(source, key).as_deref()
                            == Some("focus")
                }) && pair
                    .child_by_field_name("value")
                    .is_some_and(|value| value.kind() == "true")
                {
                    targets.push(pair);
                }
            }
        }
        for target in targets {
            let index = report(
                self,
                source,
                target.start_byte(),
                target.end_byte(),
                "Focused spec found.".into(),
                diagnostics,
            );
            let mut start = target.start_byte();
            while start > node.start_byte() && source.as_bytes()[start - 1].is_ascii_whitespace() {
                start -= 1;
            }
            if start > node.start_byte() && source.as_bytes()[start - 1] == b',' {
                start -= 1;
            }
            if preserving_replace(
                source,
                node,
                &mut corrections,
                start,
                target.end_byte(),
                "",
                self.name(),
            ) {
                diagnostics[index].corrected = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Focus, "cops/rspec/focus");
}
