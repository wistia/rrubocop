use super::super::context::*;
use std::collections::HashMap;
use std::sync::Mutex;
use tree_sitter::Node;
pub(super) fn class_equality(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((left, right)) = comparison(c, n) else {
        return vec![];
    };
    let named = name_method(c, left);
    let Some(class) = class_call(c, left, named) else {
        return vec![];
    };
    if interpolated(right) || allowed_method(c, n) {
        return vec![];
    }
    vec![class_finding(
        c,
        n,
        class,
        class_argument(c, n, right, named),
        !named || right.kind() != "string" || !c.text(right).contains('\\'),
    )]
}
fn comparison<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Node<'n>)> {
    if n.kind() == "binary" && operator(c, n) == "==" {
        field(n, "left").zip(field(n, "right"))
    } else if ["==", "eql?", "equal?"].contains(&c.method(n)) {
        field(n, "receiver").zip(args(n).first().copied())
    } else {
        None
    }
}
fn interpolated(n: Node<'_>) -> bool {
    descendants(n).any(|d| d.kind() == "interpolation")
}
fn name_method(c: &Context<'_>, n: Node<'_>) -> bool {
    ["name", "to_s", "inspect"].contains(&c.method(n))
}
fn class_call<'n>(c: &Context<'_>, n: Node<'n>, named: bool) -> Option<Node<'n>> {
    (if named { field(n, "receiver") } else { Some(n) }).filter(|cl| c.method(*cl) == "class")
}
fn class_finding(
    c: &Context<'_>,
    n: Node<'_>,
    class: Node<'_>,
    argument: Option<String>,
    correctable: bool,
) -> Finding {
    let suffix = argument
        .as_ref()
        .map(|a| format!("({a})"))
        .unwrap_or_default();
    let start = method_node(class).start_byte();
    let f = Finding::range(
        start,
        n.end_byte(),
        format!("Use `instance_of?{suffix}` instead of comparing classes."),
    );
    if argument.is_some() && correctable {
        f.edit_with(c, start, n.end_byte(), || format!("instance_of?{suffix}"))
    } else {
        f
    }
}
fn allowed_method(c: &Context<'_>, n: Node<'_>) -> bool {
    let Some(scope) = ancestors(n).find(|p| matches!(p.kind(), "method" | "singleton_method"))
    else {
        return false;
    };
    let name = c.method(scope);
    if c.config
        .options
        .get("AllowedMethods")
        .and_then(|v| v.as_sequence())
        .is_some_and(|methods| methods.iter().any(|m| m.as_str() == Some(name)))
    {
        return true;
    }
    allowed_pattern(c, name)
}
fn allowed_pattern(c: &Context<'_>, name: &str) -> bool {
    let key = pattern_key(c);
    if key.is_empty() {
        return false;
    }
    let patterns = c
        .source
        .cached(0, || AllowedPatterns(Mutex::new(HashMap::new())));
    let mut compiled = patterns.0.lock().unwrap();
    compiled
        .entry(key)
        .or_insert_with_key(|key| compile_patterns(key))
        .iter()
        .any(|regex| regex.is_match(name))
}
fn compile_patterns(patterns: &[String]) -> Vec<regex::Regex> {
    patterns
        .iter()
        .filter_map(|pattern| regex::Regex::new(pattern).ok())
        .collect()
}

fn pattern_key(c: &Context<'_>) -> Vec<String> {
    c.config
        .options
        .get("AllowedPatterns")
        .and_then(|v| v.as_sequence())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .map(str::to_owned)
        .collect()
}
struct AllowedPatterns(Mutex<HashMap<Vec<String>, Vec<regex::Regex>>>);

fn class_argument(c: &Context<'_>, n: Node<'_>, right: Node<'_>, named: bool) -> Option<String> {
    if !named {
        return Some(c.text(right).into());
    }
    if name_method(c, right) {
        return field(right, "receiver").map(|r| c.text(r).into());
    }
    if right.kind() == "string" {
        return Some(string_class(c, n, right));
    }
    if unknown_class(right) {
        None
    } else {
        Some(c.text(right).into())
    }
}
fn unknown_class(n: Node<'_>) -> bool {
    is_call(n)
        || matches!(
            n.kind(),
            "identifier" | "instance_variable" | "class_variable" | "global_variable"
        )
}
fn string_class(c: &Context<'_>, n: Node<'_>, right: Node<'_>) -> String {
    let prefix = if ancestors(n).any(|p| matches!(p.kind(), "class" | "module")) {
        "::"
    } else {
        ""
    };
    format!("{prefix}{}", c.value(right))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_patterns_keep_resolved_configs_separate() {
        let source = crate::parse::source::SourceFile::from_bytes(
            "sample.rb",
            b"def acceptable; obj.class == String; end".to_vec(),
        );
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let n = descendants(tree.root_node())
            .find(|n| n.kind() == "binary")
            .unwrap();
        assert!(allowed_method(
            &Context::new(&source, &config("^accept")),
            n
        ));
        assert!(!allowed_method(
            &Context::new(&source, &config("^other")),
            n
        ));
        assert!(!allowed_method(&Context::new(&source, &config("[")), n));
        assert!(allowed_method(
            &Context::new(&source, &config("^accept")),
            n
        ));
    }
    fn config(pattern: &str) -> crate::cop::CopConfig {
        let mut config = crate::cop::CopConfig::default();
        config.options.insert(
            "AllowedPatterns".into(),
            serde_yml::Value::Sequence(vec![serde_yml::Value::String(pattern.into())]),
        );
        config
    }
}
