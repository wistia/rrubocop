use super::super::super::context::*;
use crate::parse::source::SourceFile;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
static ACRONYM_BOUNDARY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([A-Z]+)([A-Z][a-z])").unwrap());
static WORD_BOUNDARY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([a-z\d])([A-Z])").unwrap());

pub(super) fn acronyms(root: &Path) -> Arc<Vec<String>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<Vec<String>>>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache
        .entry(root.to_owned())
        .or_insert_with(|| Arc::new(load(root)))
        .clone()
}
fn load(root: &Path) -> Vec<String> {
    let Ok(source) = SourceFile::from_path(&root.join("config/initializers/inflections.rb")) else {
        return vec![];
    };
    let Ok(tree) = crate::parse::parse_ruby(&source) else {
        return vec![];
    };
    from_tree(&source, &tree)
}
fn from_tree(source: &SourceFile, tree: &tree_sitter::Tree) -> Vec<String> {
    let c = Context {
        source,
        config: &crate::cop::CopConfig::default(),
    };
    descendants(tree.root_node())
        .into_iter()
        .filter(|n| is_call(*n) && c.method(*n) == "acronym")
        .filter_map(|n| literal_acronym(&c, n))
        .collect()
}
fn literal_acronym(c: &Context<'_>, n: tree_sitter::Node<'_>) -> Option<String> {
    let argument = args(n).into_iter().find(|a| a.kind() != "comment")?;
    (argument.kind() == "string"
        && !descendants(argument)
            .iter()
            .any(|n| n.kind() == "interpolation"))
    .then(|| c.value(argument).to_owned())
}

pub(super) fn camelize(value: &str, acronyms: &[String]) -> String {
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
fn camel_word(word: &str, acronyms: &[String]) -> String {
    if let Some(acronym) = acronyms.iter().find(|a| a.to_lowercase() == word) {
        return acronym.clone();
    }
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().chain(chars).collect()
}
pub(super) fn normalized(name: &str, acronyms: &[String]) -> String {
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
