use crate::parse::source::SourceFile;
use std::path::{Component, Path, PathBuf};

pub(super) fn resolve(source: &SourceFile) -> Option<(PathBuf, String)> {
    let path = absolute(&source.path)?;
    let project = project_root(source, &path)?;
    let relative = path.strip_prefix(&project).ok()?;
    let parts = relative.components().collect::<Vec<_>>();
    supported(&parts).then(|| (project, relative_name(&parts)))
}
fn supported(parts: &[Component<'_>]) -> bool {
    parts.len() >= 3
        && matches!(
            parts[0].as_os_str().to_str(),
            Some("app" | "not_production")
        )
}
fn relative_name(parts: &[Component<'_>]) -> String {
    let skip = 2 + usize::from(nested_root(parts));
    parts[skip..]
        .iter()
        .collect::<PathBuf>()
        .to_string_lossy()
        .into_owned()
}
fn absolute(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        Some(path.to_owned())
    } else {
        Some(std::env::current_dir().ok()?.join(path))
    }
}
fn project_root(source: &SourceFile, path: &Path) -> Option<PathBuf> {
    let directory = path.parent()?;
    source
        .operation()
        .ancestor(directory, "Gemfile", true)
        .or_else(|| source.operation().ancestor(directory, ".rubocop.yml", true))
        .or_else(|| synthetic_project(path))
}
fn synthetic_project(path: &Path) -> Option<PathBuf> {
    let parts = path.components().collect::<Vec<_>>();
    let index = parts
        .iter()
        .position(|p| matches!(p.as_os_str().to_str(), Some("app" | "not_production")))?;
    Some(parts[..index].iter().collect())
}
fn nested_root(parts: &[Component<'_>]) -> bool {
    parts[0].as_os_str() == "app"
        && parts.get(2).is_some_and(|p| {
            p.as_os_str() == "concerns"
                || parts[1].as_os_str() == "models" && p.as_os_str() == "sti_models"
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_rubocop_overrides_do_not_become_autoload_roots() {
        let project = tempfile::tempdir().unwrap();
        let nested = project.path().join("app/models/nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(project.path().join("Gemfile"), "").unwrap();
        std::fs::write(nested.join(".rubocop.yml"), "").unwrap();
        assert_eq!(
            resolve(&SourceFile::from_bytes(nested.join("widget.rb"), vec![])),
            Some((project.path().to_owned(), "nested/widget.rb".into()))
        );
    }
}
