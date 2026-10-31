use super::super::context::*;
use std::path::Path;
use tree_sitter::Node;

pub(super) fn require_autoload(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "require" {
        return vec![];
    }
    let Some(argument) = require_argument(n) else {
        return vec![];
    };
    let Some(root) = c
        .source
        .path
        .ancestors()
        .find(|p| p.join(".rubocop.yml").exists())
    else {
        return vec![];
    };
    if autoloaded(root, c.value(argument)) {
        vec![Finding::new(
            n,
            "Calling `require` on an autoloaded path breaks autoloading",
        )]
    } else {
        vec![]
    }
}

fn require_argument(n: Node<'_>) -> Option<Node<'_>> {
    args(n)
        .first()
        .copied()
        .filter(|argument| argument.kind() == "string")
}

fn autoloaded(root: &Path, required: &str) -> bool {
    ["app", "not_production"]
        .iter()
        .any(|directory| autoload_exists(&root.join(directory), required))
}

fn autoload_exists(root: &Path, required: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry.path().join(format!("{required}.rb")).exists()
            || entry
                .path()
                .join("concerns")
                .join(format!("{required}.rb"))
                .exists()
    })
}
