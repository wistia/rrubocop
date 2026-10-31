//! Register compiled extension defaults when their Ruby source is explicitly required.
use super::types::ConfigLayer;
use crate::cop::{CopConfig, EnabledState};
use crate::parse::source::SourceFile;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

pub(super) fn load_required(name: &str, working_dir: &Path, layer: &mut ConfigLayer) {
    let Some(path) = source_path(name, working_dir) else {
        return;
    };
    let Ok(source) = SourceFile::from_path(&path) else {
        return;
    };
    let Ok(tree) = crate::parse::parse_ruby(&source) else {
        return;
    };
    visit(tree.root_node(), &source, layer);
}
fn source_path(name: &str, root: &Path) -> Option<PathBuf> {
    if !name.starts_with('.') && !Path::new(name).is_absolute() && !name.ends_with(".rb") {
        return None;
    }
    Some(root.join(if name.ends_with(".rb") {
        name.to_string()
    } else {
        format!("{name}.rb")
    }))
}
fn visit(node: Node<'_>, source: &SourceFile, layer: &mut ConfigLayer) {
    if node.kind() == "class" {
        if let Some(name) = registered_name(node, source) {
            layer.cop_configs.entry(name).or_insert_with(|| CopConfig {
                enabled: EnabledState::True,
                ..CopConfig::default()
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(child, source, layer);
    }
}
fn registered_name(node: Node<'_>, source: &SourceFile) -> Option<String> {
    let declared = node_name(node, source)?;
    let short = declared.rsplit("::").next()?;
    let namespace = declared_namespace(declared).or_else(|| namespace(node, source))?;
    let candidate = format!("{namespace}/{short}");
    (super::cop_names::qualified(short)?.as_ref() == candidate).then_some(candidate)
}
fn node_name<'a>(node: Node<'_>, source: &'a SourceFile) -> Option<&'a str> {
    let name = node.child_by_field_name("name")?;
    std::str::from_utf8(&source.as_bytes()[name.byte_range()]).ok()
}
fn namespace<'a>(node: Node<'_>, source: &'a SourceFile) -> Option<&'a str> {
    let mut next = node.parent();
    while let Some(parent) = next {
        if matches!(parent.kind(), "class" | "module") {
            return node_name(parent, source)?.rsplit("::").next();
        }
        next = parent.parent();
    }
    None
}

fn declared_namespace(name: &str) -> Option<&str> {
    name.rsplit_once("::")
        .map(|(namespace, _)| namespace.rsplit("::").next().unwrap_or(namespace))
}
