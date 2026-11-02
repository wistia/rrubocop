use super::flow::Index;
use crate::cop::shared::{call_method_name, node_bytes};
use crate::model::{Entry, FileModel, IntroKind, Read, ScopeKind, Write, WriteKind};
use crate::parse::source::SourceFile;
use std::collections::HashSet;
use std::sync::Arc;

pub(super) struct Capture {
    pub name: String,
    pub scope: usize,
    pub regex: usize,
    pub expression: usize,
    pub backreference: bool,
}

pub(super) fn augment<'s>(
    source: &SourceFile,
    model: &FileModel<'s>,
    index: &Index<'_>,
) -> Option<(FileModel<'s>, Vec<Capture>)> {
    let mut expressions: Vec<_> = index
        .nodes
        .values()
        .filter(|node| {
            node.kind() == "binary"
                && node
                    .child_by_field_name("operator")
                    .is_some_and(|op| node_bytes(source, op) == b"=~")
        })
        .copied()
        .collect();
    expressions.sort_by_key(|node| node.start_byte());
    let mut groups = Vec::new();
    for expression in expressions {
        let Some(regex) = expression
            .child_by_field_name("left")
            .filter(|node| node.kind() == "regex")
        else {
            continue;
        };
        for (name, backreference) in crate::cop::ported::named_regexp_groups(source, regex) {
            groups.push((expression, regex, name, backreference));
        }
    }
    if groups.is_empty() {
        return None;
    }
    let mut scopes = model.scopes.as_ref().clone();
    let mut captures = Vec::new();
    for (expression, regex, name, backreference) in groups {
        let current = index.scope_for(expression);
        let scope = crate::model::lookup_scope(&scopes, current, regex.start_byte(), &name)
            .unwrap_or(current);
        let entry = scopes[scope]
            .entries
            .entry(name.clone().into_boxed_str())
            .or_insert_with(|| Entry {
                intro_byte: regex.start_byte(),
                intro_kind: IntroKind::Assign,
                writes: Vec::new(),
                reads: Vec::new(),
            });
        if regex.start_byte() < entry.intro_byte {
            entry.intro_byte = regex.start_byte();
            entry.intro_kind = IntroKind::Assign;
        }
        entry.writes.push(Write {
            byte: regex.start_byte(),
            node_id: regex.id(),
            kind: WriteKind::Plain,
            rhs: None,
        });
        captures.push(Capture {
            name,
            scope,
            regex: regex.id(),
            expression: expression.id(),
            backreference,
        });
    }
    let names: HashSet<_> = captures
        .iter()
        .map(|capture| capture.name.clone())
        .collect();
    for scope in 0..scopes.len() {
        if scopes[scope].kind != ScopeKind::Block {
            continue;
        }
        let merge: Vec<_> = scopes[scope]
            .entries
            .iter()
            .filter(|(name, entry)| {
                names.contains(name.as_ref())
                    && entry
                        .writes
                        .iter()
                        .any(|write| write.byte == entry.intro_byte)
            })
            .filter_map(|(name, entry)| {
                scopes[scope]
                    .parent
                    .and_then(|parent| {
                        crate::model::lookup_scope(&scopes, parent, scopes[scope].entered_at, name)
                    })
                    .map(|owner| (name.clone(), owner, entry.clone()))
            })
            .collect();
        for (name, owner, entry) in merge {
            let target = scopes[owner].entries.get_mut(&name).unwrap();
            target.writes.extend(entry.writes);
            target.reads.extend(entry.reads);
            for capture in &mut captures {
                if capture.scope == scope && capture.name.as_str() == name.as_ref() {
                    capture.scope = owner;
                }
            }
            scopes[scope].entries.remove(&name);
        }
    }
    let mut sites = model.vcall_sites.clone();
    for node in index.nodes.values() {
        if matches!(
            node.kind(),
            "variable_reference_pattern" | "expression_reference_pattern"
        ) {
            if let Some(value) = node
                .child_by_field_name("value")
                .or_else(|| node.child_by_field_name("name"))
                .filter(|node| node.kind() == "identifier")
            {
                sites.push(value.start_byte());
            }
        }
        if node.kind() == "call"
            && call_method_name(source, *node) == Some(b"binding")
            && crate::cop::shared::semantic_argument_nodes(*node).is_empty()
        {
            let scope = index.scope_for(*node);
            for name in &names {
                if let Some(owner) =
                    crate::model::lookup_scope(&scopes, scope, node.start_byte(), name)
                {
                    scopes[owner]
                        .entries
                        .get_mut(name.as_str())
                        .unwrap()
                        .reads
                        .push(Read {
                            byte: node.start_byte(),
                            under_defined: false,
                        });
                }
            }
        }
    }
    let mut converted = HashSet::new();
    sites.sort_unstable();
    sites.dedup();
    for byte in sites {
        let Some(node) = model
            .tree
            .root_node()
            .descendant_for_byte_range(byte, byte)
            .filter(|node| node.kind() == "identifier")
        else {
            continue;
        };
        if node.parent().is_some_and(|parent| {
            matches!(parent.kind(), "alias" | "undef")
                || parent.child_by_field_name("method") == Some(node)
        }) {
            continue;
        }
        let Ok(name) = std::str::from_utf8(node_bytes(source, node)) else {
            continue;
        };
        if !names.contains(name) {
            continue;
        }
        if let Some(owner) = crate::model::lookup_scope(&scopes, index.scope_for(node), byte, name)
        {
            let entry = scopes[owner].entries.get_mut(name).unwrap();
            if !entry.reads.iter().any(|read| read.byte == byte) {
                entry.reads.push(Read {
                    byte,
                    under_defined: false,
                });
            }
            converted.insert(byte);
        }
    }
    let augmented = FileModel {
        src: model.src,
        tree: model.tree.clone(),
        scopes: Arc::new(scopes),
        csend_sites: model.csend_sites.clone(),
        vcall_sites: model
            .vcall_sites
            .iter()
            .filter(|byte| !converted.contains(byte))
            .copied()
            .collect(),
    };
    Some((augmented, captures))
}
