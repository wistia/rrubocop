use super::*;

pub(in crate::cop::ported::difficult::shadow) fn reads(
    model: &FileModel<'_>,
    scope: usize,
    name: &str,
    entry: &Entry,
) -> Vec<Read> {
    let mut reads = entry
        .reads
        .iter()
        .filter(|r| !is_implicit(model, r.byte))
        .copied()
        .collect::<Vec<_>>();
    let Some(root) = at(model.tree.root_node(), model.scopes[scope].entered_at) else {
        return reads;
    };
    for node in descendants(root)
        .into_iter()
        .filter(|n| implicit_node(model, *n))
    {
        if references_argument(model, node, scope, name) {
            reads.push(Read {
                byte: node.start_byte(),
                under_defined: false,
            });
        }
    }
    reads
}
pub(in super::super) fn is_implicit(model: &FileModel<'_>, byte: usize) -> bool {
    let Some(n) = at(model.tree.root_node(), byte) else {
        return false;
    };
    bare_super(n)
        || bare_binding(model, n)
        || n.parent()
            .is_some_and(|p| p.start_byte() == byte && binding_call(model, p))
}
fn implicit_node(model: &FileModel<'_>, n: Node<'_>) -> bool {
    bare_super(n) || bare_binding(model, n) || binding_call(model, n)
}
fn bare_binding(model: &FileModel<'_>, n: Node<'_>) -> bool {
    n.kind() == "identifier"
        && model.text(n) == "binding"
        && model.vcall_sites.contains(&n.start_byte())
}
fn binding_call(model: &FileModel<'_>, n: Node<'_>) -> bool {
    is_call(n)
        && field(n, "receiver").is_none()
        && field(n, "method").is_some_and(|m| model.text(m) == "binding")
}
fn references_argument(model: &FileModel<'_>, n: Node<'_>, owner: usize, name: &str) -> bool {
    let Some(scope) = scope_at(model, n.start_byte()) else {
        return false;
    };
    if bare_super(n) {
        model.scopes[owner].kind == ScopeKind::Method
            && enclosing_method(model, scope) == Some(owner)
    } else {
        model.lookup(scope, n.start_byte(), name) == Some(owner)
    }
}
fn scope_at(model: &FileModel<'_>, byte: usize) -> Option<usize> {
    model
        .scopes
        .iter()
        .enumerate()
        .filter(|(_, scope)| {
            scope.entered_at <= byte
                && at(model.tree.root_node(), scope.entered_at).is_some_and(|n| contains(n, byte))
        })
        .max_by_key(|(_, scope)| scope.entered_at)
        .map(|(index, _)| index)
}
fn enclosing_method(model: &FileModel<'_>, mut scope: usize) -> Option<usize> {
    loop {
        if model.scopes[scope].kind == ScopeKind::Method {
            return Some(scope);
        }
        if model.scopes[scope].kind != ScopeKind::Block {
            return None;
        }
        scope = model.scopes[scope].parent?;
    }
}

fn bare_super(n: Node<'_>) -> bool {
    n.kind() == "super"
        && !n
            .parent()
            .is_some_and(|p| is_call(p) && field(p, "method") == Some(n))
}
