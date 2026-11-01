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
    let Some(required) = c.static_string(argument) else {
        return vec![];
    };
    let paths = c.source.cached(0, || project_paths(c.source));
    if paths.contains(&required) {
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

fn project_paths(source: &crate::parse::source::SourceFile) -> std::sync::Arc<AutoloadPaths> {
    let root = source
        .operation()
        .ancestor(&source.path, ".rubocop.yml", false);
    let key = root.as_deref().unwrap_or(&source.path);
    source
        .operation()
        .cached(key, || AutoloadPaths::new(root.as_deref()))
}

struct AutoloadPaths {
    roots: Vec<PathBuf>,
    resolved: Mutex<HashMap<String, bool>>,
}
impl AutoloadPaths {
    fn new(root: Option<&Path>) -> Self {
        let mut roots = Vec::new();
        if let Some(root) = root {
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
        let paths = project_paths(&source);
        assert!(!paths.contains("widget"));
        std::fs::write(project.path().join("app/models/widget.rb"), "").unwrap();
        let next = crate::parse::source::SourceFile::from_bytes(path, vec![]);
        assert!(project_paths(&next).contains("widget"));
    }
    #[test]
    fn deleted_require_targets_are_rechecked_in_the_next_operation() {
        let (project, path) = project();
        let target = project.path().join("app/models/widget.rb");
        std::fs::write(&target, "").unwrap();
        let first = crate::parse::source::SourceFile::from_bytes(&path, vec![]);
        assert!(project_paths(&first).contains("widget"));
        std::fs::remove_file(target).unwrap();
        let second = crate::parse::source::SourceFile::from_bytes(path, vec![]);
        assert!(!project_paths(&second).contains("widget"));
    }

    #[test]
    fn operation_shares_require_roots_and_negative_results_until_next_run() {
        let (project, path) = project();
        let operation = std::sync::Arc::default();
        let first = snapshot(&path, &operation);
        let second = snapshot(&path.with_file_name("second.rb"), &operation);
        let paths = project_paths(&first);
        assert!(!paths.contains("widget"));
        std::fs::write(project.path().join("app/models/widget.rb"), "").unwrap();
        assert!(std::sync::Arc::ptr_eq(&paths, &project_paths(&second)));
        assert!(!project_paths(&second).contains("widget"));
        assert!(
            project_paths(&crate::parse::source::SourceFile::from_bytes(path, vec![]))
                .contains("widget")
        );
    }

    #[test]
    fn nested_rubocop_roots_are_distinct_within_the_same_operation() {
        let (project, path) = project();
        let nested = nested_project(project.path());
        let operation = std::sync::Arc::default();
        let first = snapshot(&path, &operation);
        let second = crate::parse::source::SourceFile::from_bytes(
            nested.join("app/models/example.rb"),
            vec![],
        )
        .with_operation(operation);
        assert!(!project_paths(&first).contains("nested_only"));
        assert!(project_paths(&second).contains("nested_only"));
    }
    fn nested_project(root: &Path) -> PathBuf {
        let nested = root.join("nested");
        std::fs::create_dir_all(nested.join("app/models")).unwrap();
        std::fs::write(nested.join(".rubocop.yml"), "").unwrap();
        std::fs::write(nested.join("app/models/nested_only.rb"), "").unwrap();
        nested
    }
    fn snapshot(
        path: &Path,
        operation: &std::sync::Arc<crate::parse::source::LintOperation>,
    ) -> crate::parse::source::SourceFile {
        crate::parse::source::SourceFile::from_bytes(path, vec![]).with_operation(operation.clone())
    }
    fn project() -> (tempfile::TempDir, PathBuf) {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(".rubocop.yml"), "").unwrap();
        std::fs::create_dir_all(project.path().join("app/models")).unwrap();
        let path = project.path().join("app/models/example.rb");
        (project, path)
    }
}
