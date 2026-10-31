use super::super::context::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tree_sitter::Node;

pub(super) fn require_autoload(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "require" {
        return vec![];
    }
    let Some(argument) = require_argument(n) else {
        return vec![];
    };
    let paths = c.source.cached(0, || AutoloadPaths::new(&c.source.path));
    if paths.contains(c.value(argument)) {
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

struct AutoloadPaths {
    roots: Vec<PathBuf>,
    resolved: Mutex<HashMap<String, bool>>,
}
impl AutoloadPaths {
    fn new(path: &Path) -> Self {
        let mut roots = Vec::new();
        if let Some(root) = path.ancestors().find(|p| p.join(".rubocop.yml").exists()) {
            for directory in ["app", "not_production"] {
                if let Ok(entries) = std::fs::read_dir(root.join(directory)) {
                    for entry in entries.flatten() {
                        roots.push(entry.path());
                        roots.push(entry.path().join("concerns"));
                    }
                }
            }
        }
        Self {
            roots,
            resolved: Mutex::new(HashMap::new()),
        }
    }
    fn contains(&self, required: &str) -> bool {
        let mut resolved = self.resolved.lock().unwrap();
        *resolved.entry(required.to_owned()).or_insert_with(|| {
            let file = format!("{required}.rb");
            self.roots.iter().any(|root| root.join(&file).exists())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn autoload_paths_refresh_between_source_files() {
        let (project, path) = project();
        let source = crate::parse::source::SourceFile::from_bytes(&path, vec![]);
        let paths = source.cached(0, || AutoloadPaths::new(&path));
        assert!(!paths.contains("widget"));
        std::fs::write(project.path().join("app/models/widget.rb"), "").unwrap();
        let next = crate::parse::source::SourceFile::from_bytes(path, vec![]);
        assert!(
            next.cached(0, || AutoloadPaths::new(&next.path))
                .contains("widget")
        );
    }
    fn project() -> (tempfile::TempDir, PathBuf) {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(".rubocop.yml"), "").unwrap();
        std::fs::create_dir_all(project.path().join("app/models")).unwrap();
        let path = project.path().join("app/models/example.rb");
        (project, path)
    }
}
