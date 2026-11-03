use crate::cop::CopConfig;
use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::ruby_expression::{plain_string, plain_symbol};
use crate::cop::shared::{call_method_name, named_kids};
use crate::parse::source::SourceFile;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tree_sitter::Node;

#[derive(Default)]
pub(super) struct Schema {
    pub tables: Vec<Table>,
    pub indices: Vec<(String, Index)>,
}
pub(super) struct Table {
    pub name: String,
    pub columns: BTreeSet<String>,
    pub indices: Vec<Index>,
}
pub(super) struct Index {
    pub columns: Option<BTreeSet<String>>,
    pub expression: Option<String>,
    pub unique: bool,
}
fn statements(node: Node<'_>) -> Vec<Node<'_>> {
    let mut nodes = if matches!(node.kind(), "block_body" | "body_statement") {
        named_kids(node)
    } else {
        vec![node]
    };
    nodes.retain(|n| !matches!(n.kind(), "comment" | "heredoc_body" | "empty_statement"));
    if nodes.len() == 1 && nodes[0].kind() == "parenthesized_statements" {
        nodes = named_kids(nodes[0]);
    }
    nodes
}
fn index(source: &SourceFile, node: Node<'_>, offset: usize) -> Option<Index> {
    let args = arguments(node);
    let first = args.get(offset)?.node()?;
    let (columns, expression) = if first.kind() == "array" {
        let columns = named_kids(first)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .map(|n| plain_string(source, n))
            .collect::<Option<BTreeSet<_>>>();
        (columns, None)
    } else {
        (Some(BTreeSet::new()), plain_string(source, first))
    };
    let unique = args.last().filter(|arg| arg.hash).is_some_and(|options| {
        options.parts().iter().any(|pair| {
            pair.child_by_field_name("key")
                .and_then(|n| plain_symbol(source, n))
                .as_deref()
                == Some("unique")
        })
    });
    Some(Index {
        columns,
        expression,
        unique,
    })
}
impl Schema {
    fn parse(text: &str) -> Option<Self> {
        let source = SourceFile::from_bytes("db/schema.rb", text.as_bytes().to_vec());
        let tree = crate::parse::parse_ruby(&source).ok()?;
        if tree.root_node().has_error() {
            return None;
        }
        let roots = named_kids(tree.root_node())
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .collect::<Vec<_>>();
        if roots.len() != 1 {
            return None;
        }
        let root = roots[0];
        let body = root
            .child_by_field_name("block")?
            .child_by_field_name("body")?;
        let mut schema = Schema::default();
        for table in statements(body) {
            if call_method_name(&source, table) != Some(b"create_table") {
                continue;
            }
            let Some(block) = table.child_by_field_name("block") else {
                continue;
            };
            let args = arguments(table);
            let Some(name) = args
                .first()
                .and_then(|a| a.node())
                .and_then(|n| crate::cop::ported::original_string_value(&source, n))
            else {
                continue;
            };
            let mut entry = Table {
                name,
                columns: BTreeSet::new(),
                indices: Vec::new(),
            };
            if let Some(body) = block.child_by_field_name("body") {
                for column in statements(body) {
                    if column.child_by_field_name("block").is_some() {
                        continue;
                    }
                    if call_method_name(&source, column) == Some(b"index") {
                        if let Some(index) = index(&source, column, 0) {
                            entry.indices.push(index);
                        }
                    } else if call_method_name(&source, column).is_some() {
                        if let Some(name) = arguments(column)
                            .first()
                            .and_then(|a| a.node())
                            .and_then(|n| plain_string(&source, n))
                        {
                            entry.columns.insert(name);
                        }
                    }
                }
            }
            schema.tables.push(entry);
        }
        for (node, send) in super::migration_body::body_children(&source, body) {
            if send && call_method_name(&source, node) == Some(b"add_index") {
                if let (Some(name), Some(index)) = (
                    arguments(node)
                        .first()
                        .and_then(|a| a.node())
                        .and_then(|n| crate::cop::ported::original_string_value(&source, n)),
                    index(&source, node, 1),
                ) {
                    schema.indices.push((name, index));
                }
            }
        }
        Some(schema)
    }
}
pub(crate) fn path_from(directory: &Path) -> Option<PathBuf> {
    let mut directory = directory.to_path_buf();
    while directory.parent().is_some() {
        let path = directory.join("db/schema.rb");
        if path.exists() {
            return Some(path);
        }
        directory.pop();
    }
    None
}
pub(super) fn load(source: &SourceFile, config: &CopConfig) -> Option<Arc<Schema>> {
    #[cfg(test)]
    if let Some(value) = config.options.get("SchemaSource") {
        return value.as_str().and_then(Schema::parse).map(Arc::new);
    }
    let _ = config;
    struct SchemaSnapshot(Option<Arc<Schema>>);
    let path = path_from(&std::env::current_dir().ok()?)?;
    source
        .operation()
        .cached(&path, || {
            SchemaSnapshot(
                std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| Schema::parse(&text))
                    .map(Arc::new),
            )
        })
        .0
        .clone()
}
