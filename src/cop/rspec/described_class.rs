//! RSpec/DescribedClass — prefer `described_class` over the described constant.

use tree_sitter::Node;

use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block, is_group};
use crate::cop::shared::{call_method_name, for_each_descendant, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DescribedClass;

fn described_const<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    let first = crate::cop::shared::argument_nodes(node)
        .into_iter()
        .next()?;
    matches!(first.kind(), "constant" | "scope_resolution").then(|| node_bytes(source, first))
}

fn is_matching_const(source: &SourceFile, node: Node<'_>, want: &[u8]) -> bool {
    if !matches!(node.kind(), "constant" | "scope_resolution") {
        return false;
    }
    // `Yandex::SuggestAddresses` name node ≠ bare described `SuggestAddresses`.
    if node.kind() == "constant" {
        if let Some(parent) = node.parent() {
            if parent.kind() == "scope_resolution"
                && parent
                    .child_by_field_name("name")
                    .is_some_and(|n| n.id() == node.id())
                && parent.child_by_field_name("scope").is_some()
            {
                return false;
            }
        }
    }
    node_bytes(source, node) == want
}

fn skip_describe_arg(node: Node<'_>, describe: Node<'_>) -> bool {
    let Some(args) = describe.child_by_field_name("arguments") else {
        return false;
    };
    let mut cur = args.walk();
    args.named_children(&mut cur).any(|c| c.id() == node.id())
}

/// `Transfer` in `Transfer::FOO` — RuboCop skips when OnlyStaticConstants.
fn is_scope_prefix(node: Node<'_>) -> bool {
    node.parent().is_some_and(|p| {
        p.kind() == "scope_resolution"
            && p.child_by_field_name("scope")
                .is_some_and(|s| s.id() == node.id())
    })
}

fn is_nested_describe_arg(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(p) = node.parent() else {
        return false;
    };
    let Some(gp) = p.parent() else {
        return false;
    };
    if !matches!(gp.kind(), "call" | "command") {
        return false;
    }
    call_method_name(source, gp).is_some_and(is_group) && skip_describe_arg(node, gp)
}

/// RuboCop skips constants inside method bodies nested in the example group
/// (e.g. `def helper; Class.new { validates_with Foo }; end`).
fn inside_method(node: Node<'_>) -> bool {
    let mut p = node.parent();
    while let Some(n) = p {
        if matches!(n.kind(), "method" | "singleton_method") {
            return true;
        }
        if matches!(n.kind(), "program" | "class" | "module") {
            break;
        }
        p = n.parent();
    }
    false
}

/// RuboCop `common_instance_exec_closure?`: `Class.new` / `Module.new` / `Struct.new` blocks.
fn inside_class_new_block(source: &SourceFile, node: Node<'_>) -> bool {
    let mut p = node.parent();
    while let Some(n) = p {
        if matches!(n.kind(), "block" | "do_block") {
            if let Some(parent) = n.parent() {
                if is_class_module_struct_new(source, parent) {
                    return true;
                }
            }
        }
        if is_class_module_struct_new(source, n) {
            return true;
        }
        if matches!(n.kind(), "program" | "method" | "singleton_method") {
            break;
        }
        p = n.parent();
    }
    false
}

fn is_class_module_struct_new(source: &SourceFile, node: Node<'_>) -> bool {
    if !matches!(node.kind(), "call" | "command") {
        return false;
    }
    if call_method_name(source, node) != Some(b"new") {
        return false;
    }
    let Some(recv) = node.child_by_field_name("receiver") else {
        return false;
    };
    matches!(node_bytes(source, recv), b"Class" | b"Module" | b"Struct")
}

fn report_usage(
    cop: &DescribedClass,
    source: &SourceFile,
    n: Node<'_>,
    const_name: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(n.start_byte());
    let msg = format!(
        "Use `described_class` instead of `{}`.",
        String::from_utf8_lossy(const_name)
    );
    let mut diag = cop.diagnostic(source, line, col, msg);
    if push_replace(
        corrections,
        n.start_byte(),
        n.end_byte(),
        "described_class",
        cop.name(),
    ) {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

#[derive(Clone)]
struct AncestorExclusions {
    method: bool,
    class_new: bool,
}
struct ConstantRanges(Vec<super::cache::NodeRange>);
fn ancestor_exclusions(source: &SourceFile, node: Node<'_>) -> AncestorExclusions {
    super::cache::node_fact(source, node, || AncestorExclusions {
        method: inside_method(node),
        class_new: inside_class_new_block(source, node),
    })
}

fn reportable_const_usage(
    source: &SourceFile,
    n: Node<'_>,
    describe: Node<'_>,
    const_name: &[u8],
    only_static: bool,
) -> bool {
    is_matching_const(source, n, const_name)
        && !skip_describe_arg(n, describe)
        && !(only_static && is_scope_prefix(n))
        && !is_nested_describe_arg(source, n)
        && {
            let excluded = ancestor_exclusions(source, n);
            !excluded.method && !excluded.class_new
        }
}

impl Cop for DescribedClass {
    fn name(&self) -> &'static str {
        "RSpec/DescribedClass"
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"describe"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_str("EnforcedStyle", "described_class") != "described_class" {
            return;
        }
        let only_static = config.get_bool("OnlyStaticConstants", true);
        let Some(method) = bare_rspec_call(source, node) else {
            return;
        };
        // RuboCop `described_constant` matches only `:describe`, not x/f variants.
        if method != b"describe" {
            return;
        }
        let Some(const_name) = described_const(source, node) else {
            return;
        };
        let Some(block) = call_block(node) else {
            return;
        };
        let mut root = node;
        while let Some(parent) = root.parent() {
            root = parent;
        }
        let constants = source.cached(0, || {
            let mut ranges = Vec::new();
            for_each_descendant(root, |child| {
                if matches!(child.kind(), "constant" | "scope_resolution") {
                    ranges.push(super::cache::NodeRange::new(child));
                }
            });
            ConstantRanges(ranges)
        });
        let first = constants
            .0
            .partition_point(|range| range.start < block.start_byte());
        for range in constants.0[first..]
            .iter()
            .take_while(|range| range.start < block.end_byte())
        {
            let Some(n) = range
                .resolve(root)
                .filter(|n| n.end_byte() <= block.end_byte())
            else {
                continue;
            };
            if reportable_const_usage(source, n, node, const_name, only_static) {
                report_usage(self, source, n, const_name, diagnostics, &mut corrections);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DescribedClass, "cops/rspec/described_class");
}
