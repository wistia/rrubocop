//! Style/TrivialAccessors: original reader/writer and enclosing-scope branches.
use crate::cop::shared::{
    call_method_name, collect_comments, named_kids, node_bytes, numbered_parameter_arity,
    push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct TrivialAccessors;

const DEFAULT_ALLOWED: &[&str] = &[
    "to_ary",
    "to_a",
    "to_c",
    "to_enum",
    "to_h",
    "to_hash",
    "to_i",
    "to_int",
    "to_io",
    "to_open",
    "to_path",
    "to_proc",
    "to_r",
    "to_regexp",
    "to_str",
    "to_s",
    "to_sym",
];

fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| !matches!(n.kind(), "comment" | "empty_statement"))
        .collect()
}

fn method_body(node: Node<'_>) -> Option<Node<'_>> {
    let body = node.child_by_field_name("body")?;
    if matches!(body.kind(), "body_statement" | "block_body") {
        let parts = children(body);
        return (parts.len() == 1).then(|| parts[0]);
    }
    Some(body)
}

fn top_level(node: Node<'_>) -> bool {
    node.parent()
        .is_none_or(|p| p.kind() == "program" && children(p).len() == 1)
}

fn block_method<'a>(source: &'a SourceFile, block: Node<'_>) -> Option<&'a [u8]> {
    call_method_name(source, block.parent().filter(|n| n.kind() == "call")?)
}

fn ordinary_block(source: &SourceFile, block: Node<'_>, config: &CopConfig) -> bool {
    config.get_f64("TargetRubyVersion", 2.7) < 2.7
        || block.child_by_field_name("parameters").is_some()
        || block
            .child_by_field_name("body")
            .is_none_or(|body| numbered_parameter_arity(source, body) == 0)
}

fn excluded_scope(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        match parent.kind() {
            "class" | "singleton_class" => return false,
            "module" => return true,
            "block" | "do_block"
                if ordinary_block(source, parent, config)
                    && block_method(source, parent) == Some(b"instance_eval") =>
            {
                return true;
            }
            _ => {}
        }
        ancestor = parent.parent();
    }
    false
}

fn names_match(name: &[u8], ivar: &[u8]) -> bool {
    let name = if name.last().is_some_and(|c| matches!(c, b'=' | b'?')) {
        &name[..name.len() - 1]
    } else {
        name
    };
    ivar.strip_prefix(b"@") == Some(name)
}

fn assignment_method(name: &[u8]) -> bool {
    name.ends_with(b"=") && !matches!(name, b"==" | b"===" | b"!=" | b"<=" | b">=" | b">" | b"<")
}

fn allowed(name: &str, config: &CopConfig) -> bool {
    name == "initialize"
        || config
            .options
            .get("AllowedMethods")
            .and_then(|v| v.as_sequence())
            .map_or_else(
                || DEFAULT_ALLOWED.contains(&name),
                |values| values.iter().any(|v| v.as_str() == Some(name)),
            )
}

fn semantic_parent(node: Node<'_>) -> Option<Node<'_>> {
    let parent = node.parent()?;
    if parent.kind() == "argument_list" {
        parent.parent()
    } else {
        Some(parent)
    }
}

fn observed(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "program" | "class" | "module" | "singleton_class" => false,
        "begin" | "if" | "unless" | "conditional" | "case" | "case_match" => {
            observed(source, parent)
        }
        "block" | "do_block"
            if matches!(
                block_method(source, parent),
                Some(b"class_eval" | b"class_exec" | b"module_eval" | b"module_exec")
            ) =>
        {
            parent.parent().is_none_or(|call| observed(source, call))
        }
        "body_statement"
        | "block_body"
        | "parenthesized_statements"
        | "then"
        | "else"
        | "rescue"
        | "ensure" => children(parent).last().copied() == Some(node) && observed(source, parent),
        _ => true,
    }
}

fn unsafe_scope(source: &SourceFile, node: Node<'_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        match parent.kind() {
            "class" | "singleton_class" | "module" => return false,
            "method" | "singleton_method" => return true,
            "block" | "do_block"
                if matches!(
                    block_method(source, parent),
                    Some(b"instance_eval" | b"instance_exec")
                ) =>
            {
                return true;
            }
            _ => {}
        }
        ancestor = parent.parent();
    }
    false
}

impl Cop for TrivialAccessors {
    fn name(&self) -> &'static str {
        "Style/TrivialAccessors"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["method", "singleton_method"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if top_level(node)
            || excluded_scope(source, node, config)
            || node.kind() == "singleton_method" && config.get_bool("IgnoreClassMethods", false)
        {
            return;
        }
        let Some(method) = node.child_by_field_name("name") else {
            return;
        };
        let name = node_bytes(source, method);
        let Ok(name_text) = std::str::from_utf8(name) else {
            return;
        };
        let Some(body) = method_body(node) else {
            return;
        };
        let params = node
            .child_by_field_name("parameters")
            .map(children)
            .unwrap_or_default();
        let (kind, ivar) = if params.is_empty() && body.kind() == "instance_variable" {
            if config.get_bool("AllowPredicates", true) && name.ends_with(b"?") {
                return;
            }
            ("reader", body)
        } else if params.len() == 1
            && params[0].kind() == "identifier"
            && body.kind() == "assignment"
        {
            let (Some(left), Some(right)) = (
                body.child_by_field_name("left"),
                body.child_by_field_name("right"),
            ) else {
                return;
            };
            if left.kind() != "instance_variable"
                || right.kind() != "identifier"
                || !crate::cop::ported::ruby_local_identifier(source, right)
            {
                return;
            }
            if config.get_bool("AllowDSLWriters", true) && !assignment_method(name) {
                return;
            }
            ("writer", left)
        } else {
            return;
        };
        let match_name = names_match(name, node_bytes(source, ivar));
        if allowed(name_text, config) || config.get_bool("ExactNameMatch", true) && !match_name {
            return;
        }
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!("Use `attr_{kind}` to define trivial {kind} methods."),
        );
        diagnostic.highlight_length = 3;
        let singleton = node.kind() == "singleton_method";
        let receiver_self = node
            .child_by_field_name("object")
            .is_some_and(|n| n.kind() == "self");
        let safe = match_name
            && !(kind == "writer" && !assignment_method(name))
            && !name.ends_with(b"?")
            && (!singleton || receiver_self)
            && !semantic_parent(node).is_some_and(|p| p.kind() == "call")
            && collect_comments(node).is_empty()
            && !observed(source, node)
            && !unsafe_scope(source, node);
        if safe {
            let base = name_text.strip_suffix('=').unwrap_or(name_text);
            let accessor = format!("attr_{kind} :{base}");
            let replacement = if singleton {
                let indent = " ".repeat(column);
                format!("class << self\n{indent}  {accessor}\n{indent}end")
            } else {
                accessor
            };
            diagnostic.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                node.end_byte(),
                replacement,
                self.name(),
            );
        } else {
            diagnostic.correctable = false;
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_diagnostics_with_safeguards() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/trivial_accessors/intentional-safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&TrivialAccessors, &cases.to_string());
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &TrivialAccessors,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/trivial_accessors/parity.json"
            )),
        );
    }
}
