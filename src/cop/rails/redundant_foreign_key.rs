//! Remove explicit association keys only when the original default can be derived.
use crate::cop::shared::ruby_expression::{
    float_value, integer_ast, numbered_bindings, plain_string, plain_symbol, rational_value,
};
use crate::cop::shared::ruby_inflection::foreign_key;
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, named_kids, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct RedundantForeignKey;
fn constant_name(source: &SourceFile, node: Node<'_>) -> Option<String> {
    match node.kind() {
        "constant" => Some(node_text(source, node)),
        "scope_resolution" => {
            let name = node.child_by_field_name("name")?;
            Some(match node.child_by_field_name("scope") {
                Some(scope) => format!(
                    "{}::{}",
                    constant_name(source, scope).unwrap_or_default(),
                    node_text(source, name)
                ),
                None => node_text(source, name),
            })
        }
        _ => None,
    }
}
fn block_is_normal(source: &SourceFile, block: Node<'_>, config: &CopConfig) -> bool {
    config.get_f64("TargetRubyVersion", 3.4) < 2.7 || numbered_bindings(source, block).is_none()
}
fn new_class(source: &SourceFile, call: Node<'_>, global: bool) -> bool {
    call_method_name(source, call) == Some(b"new")
        && call_receiver(call)
            .and_then(|n| constant_name(source, n))
            .is_some_and(|name| {
                if global {
                    matches!(name.as_str(), "Class" | "Module")
                } else {
                    matches!(name.rsplit("::").next(), Some("Class" | "Module"))
                }
            })
}
fn assigned_class(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if node.kind() != "assignment" {
        return None;
    }
    let rhs = node.child_by_field_name("right")?;
    if !new_class(source, rhs, true) {
        return None;
    }
    constant_name(source, node.child_by_field_name("left")?)
}
fn parent_module(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> Option<String> {
    if node
        .child_by_field_name("block")
        .is_some_and(|b| block_is_normal(source, b, config))
    {
        return None;
    }
    let mut parts = Vec::new();
    let mut ancestor = node.parent();
    while let Some(n) = ancestor {
        match n.kind() {
            "class" | "module" => {
                if let Some(part) = n
                    .child_by_field_name("name")
                    .and_then(|n| constant_name(source, n))
                {
                    parts.push(part);
                }
            }
            "assignment" => {
                if let Some(part) = assigned_class(source, n) {
                    parts.push(part);
                }
            }
            "singleton_class" => {
                let subject = n.child_by_field_name("value")?;
                let name = if subject.kind() == "self" {
                    parent_module(source, n, config)
                } else {
                    constant_name(source, subject)
                }?;
                parts.push(format!("#<Class:{name}>"));
            }
            "lambda" if block_is_normal(source, n, config) => return None,
            "block" | "do_block" if block_is_normal(source, n, config) => {
                let owner = n.parent()?;
                if call_method_name(source, owner) == Some(b"class_eval") {
                    if let Some(receiver) = call_receiver(owner) {
                        parts.push(constant_name(source, receiver)?);
                    }
                } else if !(new_class(source, owner, false)
                    && argument_nodes(owner).is_empty()
                    && owner.parent().is_some_and(|n| {
                        n.kind() == "assignment"
                            && n.child_by_field_name("right") == Some(owner)
                            && n.child_by_field_name("left").is_some_and(|n| {
                                matches!(n.kind(), "constant" | "scope_resolution")
                            })
                    }))
                {
                    return None;
                }
            }
            _ => {}
        }
        ancestor = n.parent();
    }
    parts.reverse();
    Some(if parts.is_empty() {
        "Object".into()
    } else {
        parts.join("::")
    })
}
fn scalar(source: &SourceFile, node: Node<'_>) -> Option<String> {
    plain_symbol(source, node)
        .or_else(|| crate::cop::ported::original_string_value(source, node))
        .or_else(|| integer_ast(source, node).map(|n| n.to_string()))
        .or_else(|| float_value(source, node).map(super::pluralization_grammar::float_text))
        .or_else(|| rational_value(source, node).map(|(n, d)| format!("{n}/{d}")))
}

impl Cop for RedundantForeignKey {
    fn name(&self) -> &'static str {
        "Rails/RedundantForeignKey"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[
            b"belongs_to",
            b"has_one",
            b"has_many",
            b"has_and_belongs_to_many",
        ]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !super::unknown_env::regular(node, source)
            || call_receiver(node).is_some()
            || crate::cop::shared::is_setter_call(node)
        {
            return;
        }
        let Some(method) = call_method_name(source, node).filter(|m| {
            matches!(
                *m,
                b"belongs_to" | b"has_one" | b"has_many" | b"has_and_belongs_to_many"
            )
        }) else {
            return;
        };
        let args = argument_nodes(node);
        let Some(name) = args
            .first()
            .and_then(|n| plain_symbol(source, *n).or_else(|| plain_string(source, *n)))
        else {
            return;
        };
        let pairs = if args.len() == 2 && args[1].kind() == "hash" {
            named_kids(args[1])
        } else if args.len() > 1
            && args[1..]
                .iter()
                .all(|n| matches!(n.kind(), "pair" | "hash_splat_argument"))
        {
            args[1..].to_vec()
        } else {
            return;
        };
        let key = |p: Node<'_>| {
            p.child_by_field_name("key")
                .and_then(|n| plain_symbol(source, n))
        };
        let Some((pair, value)) = pairs
            .iter()
            .filter(|p| key(**p).as_deref() == Some("foreign_key"))
            .find_map(|p| {
                p.child_by_field_name("value")
                    .and_then(|n| plain_string(source, n).or_else(|| plain_symbol(source, n)))
                    .map(|v| (*p, v))
            })
        else {
            return;
        };
        let default = if method == b"belongs_to" {
            Some(format!("{name}_id"))
        } else if let Some(as_pair) = pairs.iter().find(|p| key(**p).as_deref() == Some("as")) {
            let Some(as_value) = as_pair.child_by_field_name("value") else {
                return;
            };
            scalar(source, as_value).map(|n| format!("{n}_id"))
        } else {
            parent_module(source, node, config).map(|n| foreign_key(&n))
        };
        if default.as_deref() != Some(&value) {
            return;
        }
        let mut start = pair.start_byte();
        let end = pair.end_byte();
        let bytes = source.as_bytes();
        while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        while start > 0 && bytes[start - 1] == b'\n' {
            start -= 1;
            if start > 0 && bytes[start - 1] == b'\r' {
                start -= 1;
            }
        }
        while start > 0 && bytes[start - 1] == b',' {
            start -= 1;
        }
        let (line, col) = source.offset_to_line_col(pair.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Specifying the default value for `foreign_key` is redundant.".into(),
        );
        d.highlight_length =
            super::blank::highlight_length(source, pair.start_byte(), pair.end_byte());
        let mut dynamic_as = false;
        for as_pair in pairs.iter().filter(|p| key(**p).as_deref() == Some("as")) {
            crate::cop::shared::for_each_descendant(*as_pair, |n| {
                dynamic_as |= n.kind() == "interpolation";
            });
        }
        d.correctable = !dynamic_as
            && !config.autocorrect_disabled()
            && !pairs.iter().any(|n| n.kind() == "hash_splat_argument")
            && ["foreign_key", "as"].iter().all(|k| {
                pairs
                    .iter()
                    .filter(|p| key(**p).as_deref() == Some(k))
                    .count()
                    <= 1
            });
        crate::cop::shared::for_each_descendant(node, |n| {
            if n.kind() == "comment" && start < n.end_byte() && end > n.start_byte() {
                d.correctable = false;
            }
        });
        if d.correctable {
            let mut proposed = bytes.to_vec();
            proposed.drain(start..end);
            let proposed = SourceFile::from_bytes("foreign_key.rb", proposed);
            d.correctable =
                crate::parse::parse_ruby(&proposed).is_ok_and(|t| !t.root_node().has_error());
        }
        if d.correctable {
            d.corrected = push_replace(&mut corrections, start, end, "", self.name());
        }
        diagnostics.push(d);
    }
}
