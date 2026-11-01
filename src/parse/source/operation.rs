use super::cache::AnalysisCache;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Hash, PartialEq, Eq)]
enum Key {
    Snapshot(PathBuf),
    Ancestor(PathBuf, &'static str, bool),
}

/// Filesystem observations shared only for one explicit lint invocation.
#[derive(Default, Debug)]
pub(crate) struct LintOperation(AnalysisCache<Key>);

impl LintOperation {
    pub(crate) fn cached<T: Send + Sync + 'static>(
        &self,
        path: &Path,
        initialize: impl FnOnce() -> T,
    ) -> Arc<T> {
        self.0.get(Key::Snapshot(path.to_owned()), initialize)
    }

    pub(crate) fn ancestor(
        &self,
        directory: &Path,
        marker: &'static str,
        file_only: bool,
    ) -> Option<PathBuf> {
        self.0
            .get(
                Key::Ancestor(directory.to_owned(), marker, file_only),
                || {
                    let path = directory.join(marker);
                    let found = if file_only {
                        path.is_file()
                    } else {
                        path.exists()
                    };
                    if found {
                        Some(directory.to_owned())
                    } else {
                        directory
                            .parent()
                            .and_then(|p| self.ancestor(p, marker, file_only))
                    }
                },
            )
            .as_ref()
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    #[test]
    fn operation_observations_share_across_files_but_not_standalone_snapshots() {
        let operation = Arc::new(LintOperation::default());
        let first = SourceFile::from_bytes("first.rb", vec![]).with_operation(operation.clone());
        let second = SourceFile::from_bytes("second.rb", vec![]).with_operation(operation);
        let key = Path::new("project");
        let original = first.operation().cached(key, || 7usize);
        assert!(Arc::ptr_eq(
            &original,
            &second.operation().cached(key, || 99usize)
        ));
        assert_eq!(
            *SourceFile::from_bytes("third.rb", vec![])
                .operation()
                .cached(key, || 42usize),
            42
        );
    }

    #[test]
    fn rebinding_an_existing_source_expires_its_old_filesystem_analysis() {
        let first = SourceFile::from_bytes("same.rb", vec![]);
        first.operation().cached(Path::new("root"), || 1usize);
        assert_eq!(*first.cached(0, || 1usize), 1);
        let next = first.with_operation(Arc::default());
        assert_eq!(*next.cached(0, || 2usize), 2);
    }

    #[test]
    fn shared_operations_keep_byte_analysis_per_source_snapshot() {
        let operation = Arc::default();
        let first = SourceFile::from_bytes("same.rb", b"one".to_vec())
            .with_operation(Arc::clone(&operation));
        let second = SourceFile::from_bytes("same.rb", b"two".to_vec()).with_operation(operation);
        assert_eq!(*first.cached(0, || 1usize), 1);
        assert_eq!(*second.cached(0, || 2usize), 2);
    }

    #[test]
    fn marker_queries_keep_exists_and_regular_file_policies_separate() {
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir(project.path().join(".rubocop.yml")).unwrap();
        let operation = LintOperation::default();
        assert_eq!(
            operation.ancestor(project.path(), ".rubocop.yml", false),
            Some(project.path().to_owned())
        );
        assert_eq!(
            operation.ancestor(project.path(), ".rubocop.yml", true),
            None
        );
    }

    #[test]
    fn removed_project_markers_are_rechecked_in_the_next_operation() {
        let project = tempfile::tempdir().unwrap();
        let marker = project.path().join("Gemfile");
        std::fs::write(&marker, "").unwrap();
        let operation = LintOperation::default();
        assert!(
            operation
                .ancestor(project.path(), "Gemfile", true)
                .is_some()
        );
        std::fs::remove_file(marker).unwrap();
        assert!(
            LintOperation::default()
                .ancestor(project.path(), "Gemfile", true)
                .is_none()
        );
    }

    #[test]
    fn marker_snapshots_expire_for_the_next_lint_operation() {
        let project = tempfile::tempdir().unwrap();
        let operation = LintOperation::default();
        assert_eq!(operation.ancestor(project.path(), "Gemfile", true), None);
        std::fs::write(project.path().join("Gemfile"), "").unwrap();
        assert_eq!(operation.ancestor(project.path(), "Gemfile", true), None);
        assert_eq!(
            LintOperation::default().ancestor(project.path(), "Gemfile", true),
            Some(project.path().to_owned())
        );
    }
}
