//! Rails/SkipsModelValidations — methods that skip Active Record validations.

use tree_sitter::Node;

use crate::cop::shared::ruby_arguments::{Argument, arguments};
use crate::cop::shared::ruby_expression::{plain_symbol, regular_send_name, send_name};
use crate::cop::shared::{call_receiver, method_node, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SkipsModelValidations;

const NEEDS_ARGS: &[&str] = &[
    "decrement!",
    "decrement_counter",
    "increment!",
    "increment_counter",
    "insert",
    "insert!",
    "insert_all",
    "insert_all!",
    "toggle!",
    "update_all",
    "update_attribute",
    "update_column",
    "update_columns",
    "update_counters",
    "upsert",
    "upsert_all",
];

fn in_list(config: &CopConfig, key: &str, name: &str) -> bool {
    config
        .options
        .get(key)
        .and_then(|v| v.as_sequence())
        .is_some_and(|items| items.iter().any(|v| v.as_str() == Some(name)))
}

fn forbidden(config: &CopConfig, name: &str) -> bool {
    if config.options.get("Blacklist").is_some_and(|value| {
        !matches!(
            value,
            serde_yml::Value::Null | serde_yml::Value::Bool(false)
        )
    }) {
        return in_list(config, "Blacklist", name);
    }
    if config.options.contains_key("ForbiddenMethods") {
        return in_list(config, "ForbiddenMethods", name);
    }
    matches!(name, "touch" | "touch_all") || NEEDS_ARGS.contains(&name)
}

fn allowed(config: &CopConfig, name: &str) -> bool {
    if config.options.get("Whitelist").is_some_and(|value| {
        !matches!(
            value,
            serde_yml::Value::Null | serde_yml::Value::Bool(false)
        )
    }) {
        return in_list(config, "Whitelist", name);
    }
    in_list(config, "AllowedMethods", name)
}

fn file_utils(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "constant" => node_bytes(source, node) == b"FileUtils",
        "scope_resolution" => {
            node.child_by_field_name("scope").is_none()
                && node
                    .child_by_field_name("name")
                    .is_some_and(|n| node_bytes(source, n) == b"FileUtils")
        }
        _ => false,
    }
}

fn good_touch(source: &SourceFile, node: Node<'_>, name: &str) -> bool {
    if name != "touch" || regular_send_name(source, node).is_none() {
        return false;
    }
    if call_receiver(node).is_some_and(|r| file_utils(source, r)) {
        return true;
    }
    let args = arguments(node);
    args.len() == 1
        && args[0]
            .node()
            .is_some_and(|node| matches!(node.kind(), "true" | "false"))
}
fn good_insert(source: &SourceFile, args: &[Argument<'_>], name: &str) -> bool {
    if !matches!(name, "insert" | "insert!") || args.len() < 2 {
        return false;
    }
    !args[1].hash
        || args[1].parts().into_iter().any(|pair| {
            pair.kind() == "pair"
                && pair
                    .child_by_field_name("key")
                    .and_then(|key| plain_symbol(source, key))
                    .is_some_and(|key| !matches!(key.as_str(), "returning" | "unique_by"))
        })
}
fn push_offense(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (start, end) = if node.kind() == "element_reference" {
        let mut cursor = node.walk();
        let brackets = node
            .children(&mut cursor)
            .filter(|child| matches!(child.kind(), "[" | "]"))
            .collect::<Vec<_>>();
        let (Some(first), Some(last)) = (brackets.first(), brackets.last()) else {
            return;
        };
        (first.start_byte(), last.end_byte())
    } else {
        let selector = method_node(node)
            .or_else(|| node.child_by_field_name("operator"))
            .unwrap_or(node);
        (selector.start_byte(), selector.end_byte())
    };
    let (line, col) = source.offset_to_line_col(start);
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        format!("Avoid using `{name}` because it skips validations."),
    );
    diag.highlight_length = super::blank::highlight_length(source, start, end);
    diagnostics.push(diag);
}

impl Cop for SkipsModelValidations {
    fn name(&self) -> &'static str {
        "Rails/SkipsModelValidations"
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[
            "call",
            "identifier",
            "hash_key_symbol",
            "binary",
            "unary",
            "element_reference",
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(bytes) = send_name(source, node) else {
            return;
        };
        let Ok(name) = std::str::from_utf8(&bytes) else {
            return;
        };
        let args = arguments(node);
        if allowed(config, name)
            || !forbidden(config, name)
            || (NEEDS_ARGS.contains(&name) && args.is_empty())
            || good_touch(source, node, name)
            || good_insert(source, &args, name)
        {
            return;
        }
        push_offense(self, source, node, name, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SkipsModelValidations, "cops/rails/skips_model_validations");

    fn list_config(key: &str, names: &[&str]) -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            key.into(),
            serde_yml::Value::Sequence(
                names
                    .iter()
                    .map(|n| serde_yml::Value::String((*n).into()))
                    .collect(),
            ),
        );
        config
    }

    #[test]
    fn allowed_methods_supersede_forbidden() {
        let mut config = list_config("ForbiddenMethods", &["toggle!", "touch"]);
        config.options.insert(
            "AllowedMethods".into(),
            serde_yml::Value::Sequence(vec![serde_yml::Value::String("touch".into())]),
        );
        crate::testutil::assert_cop_offenses_full_with_config(
            &SkipsModelValidations,
            b"user.toggle!(:active)\n     ^^^^^^^ Rails/SkipsModelValidations: Avoid using `toggle!` because it skips validations.\n",
            config.clone(),
        );
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &SkipsModelValidations,
            b"User.touch(:attr)\n",
            config,
        );
    }

    #[test]
    fn blacklist_is_the_forbidden_list() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &SkipsModelValidations,
            b"others.update_all(current: false)\n",
            list_config("Blacklist", &["toggle!"]),
        );
    }
}
