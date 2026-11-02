//! Scope-introducing constructs: method/class bodies, blocks and lambdas.

use tree_sitter::Node;

use super::builder::{Builder, body_of};
use super::{ScopeId, ScopeKind};

impl Builder<'_> {
    /// Enter a fresh scope and walk only its parameters and body.
    /// Returns true when `kind` introduced a scope.
    pub(super) fn walk_scope_intro(&mut self, n: Node, kind: &str, parent: ScopeId) -> bool {
        match kind {
            "method" | "singleton_method" => {
                // `def obj.foo` reads `obj` in the enclosing scope.
                if kind == "singleton_method" {
                    walk_singleton_object(self, n, parent);
                }
                let s = self.scope_for(n, ScopeKind::Method, None);
                self.declare_params_and_body(n, s);
            }
            "class" | "module" | "singleton_class" => {
                // `class << obj` reads `obj` in the enclosing scope.
                if kind == "singleton_class" {
                    walk_singleton_class_object(self, n, parent);
                } else if let Some(sc) = n.child_by_field_name("superclass") {
                    self.walk(sc, parent, false);
                }
                if matches!(kind, "class" | "module") {
                    if let Some(name) = n.child_by_field_name("name") {
                        self.walk(name, parent, false);
                    }
                }
                let s = self.scope_for(n, ScopeKind::ClassLike, None);
                if let Some(body) = n.child_by_field_name("body") {
                    self.walk(body, s, false);
                }
            }
            "block" | "do_block" | "lambda" => {
                let s = self.scope_for(n, ScopeKind::Block, Some(parent));
                self.declare_block_params_and_body(n, s);
            }
            _ => return false,
        }
        true
    }

    fn declare_params_and_body(&mut self, n: Node, s: ScopeId) {
        if let Some(p) = n.child_by_field_name("parameters") {
            self.declare_params(p, s);
        }
        if let Some(body) = n.child_by_field_name("body") {
            self.walk(body, s, false);
        }
    }

    fn declare_block_params_and_body(&mut self, n: Node, s: ScopeId) {
        if let Some(p) = n.child_by_field_name("parameters") {
            self.declare_params(p, s); // block params always shadow
        }
        if let Some(body) = body_of(n) {
            if n.child_by_field_name("parameters").is_none() {
                self.declare_numbered_params(body, s);
            }
            self.walk(body, s, false);
        }
    }

    fn declare_numbered_params(&mut self, node: Node<'_>, scope: ScopeId) {
        let headers: Option<Vec<Node<'_>>> = match node.kind() {
            "block" | "do_block" | "lambda" | "method" => Some(Vec::new()),
            "singleton_method" => Some(node.child_by_field_name("object").into_iter().collect()),
            "singleton_class" => Some(node.child_by_field_name("value").into_iter().collect()),
            "class" | "module" => Some(
                [
                    node.child_by_field_name("name"),
                    node.child_by_field_name("superclass"),
                ]
                .into_iter()
                .flatten()
                .collect(),
            ),
            _ => None,
        };
        if let Some(headers) = headers {
            for header in headers {
                self.declare_numbered_params(header, scope);
            }
            return;
        }
        if node.kind() == "identifier" {
            let text = &self.src[node.start_byte()..node.end_byte()];
            let method_name = node.parent().is_some_and(|parent| {
                parent.child_by_field_name("method") == Some(node)
                    || matches!(parent.kind(), "alias" | "undef")
            });
            if matches!(text, [b'_', b'1'..=b'9']) && !method_name {
                self.bind_param(
                    scope,
                    String::from_utf8_lossy(text).into_owned(),
                    node.start_byte(),
                );
            }
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.declare_numbered_params(child, scope);
        }
    }

    pub(super) fn declare_params(&mut self, container: Node, scope: ScopeId) {
        let mut cursor = container.walk();
        for child in container.children(&mut cursor) {
            match child.kind() {
                "," | "(" | ")" | "|" => {}
                "identifier" | "optional_parameter" | "keyword_parameter" | "block_parameter"
                | "splat_parameter" => self.declare_named_param(child, scope),
                _ => self.declare_wrapped_param(child, scope),
            }
        }
    }

    /// A named parameter binds directly; default/kw values may contain
    /// arbitrary expressions and are walked.
    fn declare_named_param(&mut self, child: Node, scope: ScopeId) {
        if let Some(v) = child.child_by_field_name("value") {
            self.walk(v, scope, false);
        }
        if let Some(name) = super::builder::declared_name(child, self.src) {
            self.bind_param(scope, name, child.start_byte());
        }
    }

    /// Parameter wrappers recurse through bindings, never through default expressions.
    fn declare_wrapped_param(&mut self, child: Node, scope: ScopeId) {
        if matches!(
            child.kind(),
            "identifier"
                | "optional_parameter"
                | "keyword_parameter"
                | "block_parameter"
                | "splat_parameter"
        ) {
            self.declare_named_param(child, scope);
            return;
        }
        let mut sub = child.walk();
        for inner in child.named_children(&mut sub) {
            self.declare_wrapped_param(inner, scope);
        }
    }
}

/// Walk the object of `class << obj` in the enclosing scope (a read).
fn walk_singleton_class_object(b: &mut Builder<'_>, n: Node<'_>, parent: ScopeId) {
    if let Some(obj) = n.child_by_field_name("value") {
        b.walk(obj, parent, false);
        return;
    }
    // tree-sitter-ruby: expression after `<<`.
    let mut cur = n.walk();
    let mut seen_shl = false;
    for child in n.children(&mut cur) {
        if child.kind() == "<<" {
            seen_shl = true;
            continue;
        }
        if !seen_shl {
            continue;
        }
        if matches!(
            child.kind(),
            "identifier"
                | "constant"
                | "self"
                | "instance_variable"
                | "call"
                | "parenthesized_statements"
        ) {
            b.walk(child, parent, false);
            break;
        }
    }
}

/// Walk the object of `def obj.method` in the enclosing scope (a read).
fn walk_singleton_object(b: &mut Builder<'_>, n: Node<'_>, parent: ScopeId) {
    if let Some(obj) = n.child_by_field_name("object") {
        b.walk(obj, parent, false);
        return;
    }
    // tree-sitter-ruby: first expression after `def`, before `.`.
    let mut cur = n.walk();
    let mut seen_def = false;
    for child in n.children(&mut cur) {
        if child.kind() == "def" {
            seen_def = true;
            continue;
        }
        if !seen_def {
            continue;
        }
        if child.kind() == "." {
            break;
        }
        if matches!(
            child.kind(),
            "identifier"
                | "constant"
                | "self"
                | "instance_variable"
                | "class_variable"
                | "global_variable"
                | "parenthesized_statements"
                | "call"
        ) {
            b.walk(child, parent, false);
            break;
        }
    }
}
