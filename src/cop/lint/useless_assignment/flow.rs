use crate::cop::shared::node_bytes;
use crate::model::{Entry, FileModel, ScopeKind, Write};
use crate::parse::source::SourceFile;
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

pub(super) struct Index<'t> {
    pub nodes: HashMap<usize, Node<'t>>,
    reads: HashMap<usize, usize>,
    writes: HashMap<usize, usize>,
    anchors: Vec<Node<'t>>,
    loops: Vec<Node<'t>>,
}
fn kids(node: Node<'_>) -> Vec<Node<'_>> {
    let mut c = node.walk();
    node.named_children(&mut c).collect()
}
fn scope_kind(node: Node<'_>) -> Option<ScopeKind> {
    match node.kind() {
        "method" | "singleton_method" => Some(ScopeKind::Method),
        "class" | "module" | "singleton_class" => Some(ScopeKind::ClassLike),
        "block" | "do_block" | "lambda" => Some(ScopeKind::Block),
        _ => None,
    }
}
impl<'t> Index<'t> {
    pub fn new(model: &FileModel<'_>, root: Node<'t>) -> Self {
        let mut index = Self {
            nodes: HashMap::new(),
            reads: HashMap::new(),
            writes: HashMap::new(),
            anchors: vec![root; model.scopes.len()],
            loops: Vec::new(),
        };
        index.walk(root, &mut 0);
        for (id, scope) in model.scopes.iter().enumerate().skip(1) {
            if let Some(node) = index.nodes.values().find(|node| {
                node.start_byte() == scope.entered_at && scope_kind(**node) == Some(scope.kind)
            }) {
                index.anchors[id] = *node;
            }
        }
        index
    }
    fn walk(&mut self, node: Node<'t>, rank: &mut usize) {
        self.nodes.insert(node.id(), node);
        self.reads.entry(node.start_byte()).or_insert(*rank);
        *rank += 1;
        if matches!(
            node.kind(),
            "while" | "until" | "while_modifier" | "until_modifier" | "for"
        ) || (matches!(node.kind(), "begin" | "body_statement")
            && kids(node).iter().any(|n| n.kind() == "rescue")
            && {
                let mut retry = false;
                crate::cop::shared::for_each_descendant(node, |n| retry |= n.kind() == "retry");
                retry
            })
        {
            self.loops.push(node);
        }
        if matches!(node.kind(), "assignment" | "operator_assignment")
            && node
                .child_by_field_name("left")
                .is_some_and(|left| matches!(left.kind(), "identifier" | "left_assignment_list"))
        {
            if let Some(rhs) = node.child_by_field_name("right") {
                self.walk(rhs, rank);
            }
            if let Some(lhs) = node.child_by_field_name("left") {
                self.walk_targets(lhs, rank);
            }
        } else if matches!(
            node.kind(),
            "if_modifier" | "unless_modifier" | "while_modifier" | "until_modifier"
        ) {
            let body = node.child_by_field_name("body");
            let condition = node.child_by_field_name("condition");
            let post = matches!(node.kind(), "while_modifier" | "until_modifier")
                && body.is_some_and(|n| n.kind() == "begin");
            for child in if post {
                [body, condition]
            } else {
                [condition, body]
            }
            .into_iter()
            .flatten()
            {
                self.walk(child, rank);
            }
        } else {
            for child in kids(node) {
                self.walk(child, rank);
            }
        }
        self.reads.entry(node.end_byte()).or_insert(*rank);
        *rank += 1;
    }
    fn walk_targets(&mut self, node: Node<'t>, rank: &mut usize) {
        self.nodes.insert(node.id(), node);
        self.reads.entry(node.start_byte()).or_insert(*rank);
        *rank += 1;
        for child in kids(node) {
            self.walk_targets(child, rank);
        }
        self.writes.insert(node.id(), *rank);
        *rank += 1;
    }
    pub fn scope_node(&self, id: usize) -> Node<'t> {
        self.anchors[id]
    }
    pub fn belongs(&self, node: Node<'_>, scope: usize) -> bool {
        let mut current = Some(node);
        while let Some(n) = current {
            if let Some(kind) = scope_kind(n) {
                let header = match n.kind() {
                    "class" | "module" => ["name", "superclass"].iter().any(|field| {
                        n.child_by_field_name(field).is_some_and(|part| {
                            part.start_byte() <= node.start_byte()
                                && node.end_byte() <= part.end_byte()
                        })
                    }),
                    "singleton_method" => n.child_by_field_name("object").is_some_and(|part| {
                        part.start_byte() <= node.start_byte() && node.end_byte() <= part.end_byte()
                    }),
                    _ => false,
                };
                if !header {
                    return self.anchors[scope] == n
                        && scope_kind(self.anchors[scope]) == Some(kind);
                }
            }
            current = n.parent();
        }
        scope == 0
    }
    pub fn names(&self, source: &SourceFile, model: &FileModel<'_>, scope: usize) -> Vec<String> {
        let mut sites: Vec<_> = model
            .vcall_sites
            .iter()
            .filter_map(|byte| {
                let node = self.read_node(self.anchors[0], *byte);
                self.belongs(node, scope).then_some((
                    node.start_byte(),
                    String::from_utf8_lossy(node_bytes(source, node)).into_owned(),
                ))
            })
            .collect();
        for node in self
            .nodes
            .values()
            .filter(|node| node.kind() == "call" && self.belongs(**node, scope))
        {
            if crate::cop::shared::call_receiver(*node).is_none()
                && crate::cop::shared::semantic_argument_nodes(*node).is_empty()
            {
                if let Some(name) = crate::cop::shared::call_method_name(source, *node) {
                    sites.push((
                        node.start_byte(),
                        String::from_utf8_lossy(name).into_owned(),
                    ));
                }
            }
        }
        sites.sort_by_key(|(byte, _)| *byte);
        let mut vars: Vec<_> = model.scopes[scope].entries.iter().collect();
        vars.sort_by_key(|(_, entry)| entry.intro_byte);
        let mut names: Vec<_> = sites.into_iter().map(|(_, name)| name).collect();
        names.extend(vars.into_iter().map(|(name, _)| name.to_string()));
        let mut seen = HashSet::new();
        names.retain(|name| seen.insert(name.clone()));
        names
    }
    pub fn scope_for(&self, node: Node<'_>) -> usize {
        (0..self.anchors.len())
            .find(|scope| self.belongs(node, *scope))
            .unwrap_or(0)
    }
    pub fn register_capture(&mut self, regex: usize, expression: usize) {
        if let Some(node) = self.nodes.get(&expression) {
            if let Some(rank) = self.reads.get(&node.end_byte()) {
                self.writes.insert(regex, *rank);
            }
        }
    }
    pub fn write_rank(&self, write: &Write) -> usize {
        self.writes
            .get(&write.node_id)
            .copied()
            .unwrap_or_else(|| self.reads.get(&write.byte).copied().unwrap_or(write.byte))
    }
    fn read_node(&self, root: Node<'t>, byte: usize) -> Node<'t> {
        let mut node = root.descendant_for_byte_range(byte, byte).unwrap_or(root);
        while !node.is_named() {
            if let Some(parent) = node.parent() {
                node = parent;
            } else {
                break;
            }
        }
        node
    }
    fn captured(&self, scope: usize, node: Node<'t>) -> bool {
        let anchor = self.anchors[scope];
        let mut current = Some(node);
        while let Some(n) = current {
            if n == anchor {
                return false;
            }
            if matches!(n.kind(), "block" | "do_block" | "lambda") {
                return true;
            }
            if matches!(
                n.kind(),
                "method" | "singleton_method" | "class" | "module" | "singleton_class"
            ) {
                return false;
            }
            current = n.parent();
        }
        false
    }
    pub fn unused(
        &self,
        source: &SourceFile,
        model: &FileModel<'_>,
        scope: usize,
        entry: &Entry,
    ) -> Vec<Write> {
        let root = *self.anchors.first().unwrap();
        let anchor = self.anchors[scope];
        if entry.writes.iter().any(|write| {
            self.nodes
                .get(&write.node_id)
                .is_some_and(|node| self.captured(scope, *node))
        }) || entry
            .reads
            .iter()
            .any(|read| self.captured(scope, self.read_node(root, read.byte)))
        {
            return Vec::new();
        }
        let mut events: Vec<_> = entry
            .writes
            .iter()
            .enumerate()
            .map(|(id, w)| (self.write_rank(w), Some(id), w.byte))
            .collect();
        events.extend(entry.reads.iter().map(|read| {
            (
                self.reads.get(&read.byte).copied().unwrap_or(read.byte),
                None,
                read.byte,
            )
        }));
        events.sort_by_key(|(rank, write, _)| (*rank, write.is_some()));
        let mut assignments = Vec::new();
        let mut used = HashSet::new();
        for (_, write, byte) in events {
            if let Some(id) = write {
                assignments.push(id);
                continue;
            }
            let reference = branch_path(source, self.read_node(root, byte), anchor);
            let mut consumed = HashSet::new();
            for id in assignments.iter().rev().copied() {
                let assignment = entry.writes[id];
                let Some(node) = self.nodes.get(&assignment.node_id).copied() else {
                    continue;
                };
                let branches = branch_path(source, node, anchor);
                if branches
                    .first()
                    .is_some_and(|branch| consumed.contains(&branch.key))
                {
                    continue;
                }
                if !exclusive(&branches, &reference) {
                    used.insert(id);
                }
                if modifier_assignment(node) {
                    continue;
                }
                if branches.is_empty()
                    || branches.first().map(|b| b.key) == reference.first().map(|b| b.key)
                {
                    break;
                }
                if let Some(branch) = branches.first().filter(|branch| !branch.incomplete) {
                    consumed.insert(branch.key);
                }
            }
        }
        for (id, write) in entry.writes.iter().enumerate() {
            let Some(node) = self.nodes.get(&write.node_id) else {
                continue;
            };
            for loop_node in &self.loops {
                if !(loop_node.start_byte() <= node.start_byte()
                    && node.end_byte() <= loop_node.end_byte())
                    || !within_scope(*loop_node, anchor)
                {
                    continue;
                }
                if entry.reads.iter().any(|read| {
                    loop_node.start_byte() <= read.byte && read.byte < loop_node.end_byte()
                }) {
                    used.insert(id);
                }
            }
        }
        let _ = model;
        entry
            .writes
            .iter()
            .enumerate()
            .filter(|(id, _)| !used.contains(id))
            .map(|(_, write)| *write)
            .collect()
    }
}
fn within_scope(node: Node<'_>, anchor: Node<'_>) -> bool {
    let mut current = Some(node);
    while let Some(n) = current {
        if n == anchor {
            return true;
        }
        if scope_kind(n).is_some() {
            return false;
        }
        current = n.parent();
    }
    false
}
fn modifier_assignment(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    if parent.is_some_and(|n| matches!(n.kind(), "assignment" | "operator_assignment")) {
        parent = parent.unwrap().parent();
    }
    if parent.is_some_and(|n| n.kind() == "parenthesized_statements") {
        parent = parent.unwrap().parent();
    }
    parent.is_some_and(|p| {
        matches!(
            p.kind(),
            "if_modifier" | "unless_modifier" | "while_modifier" | "until_modifier"
        )
    })
}
#[derive(Clone, Copy)]
struct Branch {
    key: (usize, u8, usize),
    jump: bool,
    incomplete: bool,
}
fn branch_path(source: &SourceFile, node: Node<'_>, anchor: Node<'_>) -> Vec<Branch> {
    let mut result = Vec::new();
    let mut child = node;
    while child != anchor {
        let Some(parent) = child.parent() else {
            break;
        };
        let kind = parent.kind();
        let conditional = matches!(
            kind,
            "if" | "unless"
                | "elsif"
                | "conditional"
                | "if_modifier"
                | "unless_modifier"
                | "while"
                | "until"
                | "while_modifier"
                | "until_modifier"
        );
        let mut part = None;
        if conditional && parent.child_by_field_name("condition") != Some(child) {
            part = Some(child.id());
        } else if kind == "for" && parent.child_by_field_name("body") == Some(child) {
            part = Some(child.id());
        } else if matches!(kind, "case" | "case_match")
            && parent.child_by_field_name("value") != Some(child)
        {
            part = Some(child.id());
        } else if kind == "binary"
            && parent.child_by_field_name("right") == Some(child)
            && parent
                .child_by_field_name("operator")
                .is_some_and(|op| matches!(node_bytes(source, op), b"&&" | b"||" | b"and" | b"or"))
        {
            part = Some(child.id());
        }
        if let Some(part) = part {
            result.push(Branch {
                key: (parent.id(), 0, part),
                jump: false,
                incomplete: false,
            });
        }
        if matches!(kind, "begin" | "body_statement") {
            let children = kids(parent);
            let rescue = children.iter().any(|n| n.kind() == "rescue");
            let ensure = children.iter().any(|n| n.kind() == "ensure");
            if child.kind() != "ensure" {
                if rescue {
                    let handler = matches!(child.kind(), "rescue" | "else");
                    result.push(Branch {
                        key: (
                            parent.id(),
                            1,
                            if handler { child.id() } else { parent.id() },
                        ),
                        jump: !handler,
                        incomplete: !handler,
                    });
                }
                if ensure {
                    result.push(Branch {
                        key: (parent.id(), 2, parent.id()),
                        jump: true,
                        incomplete: true,
                    });
                }
            }
        }
        child = parent;
    }
    result
}
fn exclusive(assignment: &[Branch], reference: &[Branch]) -> bool {
    for branch in assignment {
        if branch.jump {
            return false;
        }
        if let Some(other) = reference
            .iter()
            .find(|other| other.key.0 == branch.key.0 && other.key.1 == branch.key.1)
        {
            return branch.key.2 != other.key.2;
        }
    }
    false
}
