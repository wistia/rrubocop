use super::super::super::context::*;
use crate::parse::source::SourceFile;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
static ACRONYM_BOUNDARY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([A-Z]+)([A-Z][a-z])").unwrap());
static WORD_BOUNDARY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([a-z\d])([A-Z])").unwrap());

pub(super) type Acronyms = Vec<(String, String)>;
type CachedAcronyms = (Vec<u8>, Arc<Acronyms>);
pub(super) fn acronyms(root: &Path) -> Arc<Acronyms> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedAcronyms>>> = OnceLock::new();
    let content = initializer_content(root);
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    if let Some((_, acronyms)) = cache.get(root).filter(|(previous, _)| *previous == content) {
        return acronyms.clone();
    }
    let acronyms = Arc::new(load(root, content.clone()));
    cache.insert(root.to_owned(), (content, acronyms.clone()));
    acronyms
}
fn initializer_content(root: &Path) -> Vec<u8> {
    std::fs::read(root.join("config/initializers/inflections.rb")).unwrap_or_default()
}
fn load(root: &Path, content: Vec<u8>) -> Acronyms {
    let source = SourceFile::from_bytes(root.join("config/initializers/inflections.rb"), content);
    crate::parse::parse_ruby(&source)
        .map(|tree| {
            from_tree(&source, &tree)
                .into_iter()
                .map(|name| (name.to_lowercase(), name))
                .collect()
        })
        .unwrap_or_default()
}
fn from_tree(source: &SourceFile, tree: &tree_sitter::Tree) -> Vec<String> {
    let config = crate::cop::CopConfig::default();
    let c = Context::new(source, &config);
    descendants(tree.root_node())
        .filter(|n| is_call(*n) && c.method(*n) == "acronym")
        .filter_map(|n| literal_acronym(&c, n))
        .collect()
}
fn literal_acronym(c: &Context<'_>, n: tree_sitter::Node<'_>) -> Option<String> {
    let argument = args(n).into_iter().find(|a| a.kind() != "comment")?;
    (argument.kind() == "string" && !descendants(argument).any(|n| n.kind() == "interpolation"))
        .then(|| c.value(argument).to_owned())
}

pub(super) fn camelize(value: &str, acronyms: &Acronyms) -> String {
    value
        .split('/')
        .map(|part| {
            part.split('_')
                .map(|word| camel_word(word, acronyms))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("::")
}
fn camel_word(word: &str, acronyms: &Acronyms) -> String {
    if let Some((_, acronym)) = acronyms.iter().find(|(lower, _)| lower == word) {
        return acronym.clone();
    }
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().chain(chars).collect()
}
pub(super) fn normalized(name: &str, acronyms: &Acronyms) -> String {
    if camelize(&underscore(name), acronyms) == name {
        name.into()
    } else {
        "APlaceHolderName".into()
    }
}

fn underscore(name: &str) -> String {
    WORD_BOUNDARY
        .replace_all(
            &ACRONYM_BOUNDARY.replace_all(name, "${1}_${2}"),
            "${1}_${2}",
        )
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inflections_refresh_when_initializer_changes() {
        let project = tempfile::tempdir().unwrap();
        let directory = project.path().join("config/initializers");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("inflections.rb");
        assert!(acronyms(project.path()).is_empty());
        std::fs::write(&path, "inflect.acronym 'API'").unwrap();
        assert_eq!(
            camelize("api_client", &acronyms(project.path())),
            "APIClient"
        );
        std::fs::write(&path, "inflect.acronym 'OAuth'").unwrap();
        assert_eq!(
            camelize("oauth_client", &acronyms(project.path())),
            "OAuthClient"
        );
        assert_eq!(
            camelize("api_client", &acronyms(project.path())),
            "ApiClient"
        );
    }
    #[test]
    fn deleted_inflections_are_not_cached() {
        let project = tempfile::tempdir().unwrap();
        let directory = project.path().join("config/initializers");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("inflections.rb");
        std::fs::write(&path, "inflect.acronym 'API'").unwrap();
        assert!(!acronyms(project.path()).is_empty());
        std::fs::remove_file(path).unwrap();
        assert!(acronyms(project.path()).is_empty());
    }
}
