use super::*;
use std::collections::{HashMap, HashSet};

type Site = (usize, usize, bool);
type Range = Option<(usize, usize)>;
pub(in crate::cop::ported::difficult::shadow) struct Index {
    pub(super) implicit: HashSet<usize>,
    pub(super) explicit: HashMap<String, Vec<usize>>,
    sites: Vec<Site>,
    ranges: Vec<Range>,
}
impl Index {
    pub(in crate::cop::ported::difficult::shadow) fn new(model: &FileModel<'_>) -> Self {
        let ranges = scope_ranges(model);
        let vcalls = model.vcall_sites.iter().copied().collect::<HashSet<_>>();
        let sites = implicit_sites(model, &ranges, &vcalls);
        let implicit = sites
            .iter()
            .map(|s| s.0)
            .filter(|byte| implicit_read(model, *byte, &vcalls))
            .collect();
        let explicit = explicit_reads(model, &implicit);
        Self {
            implicit,
            explicit,
            sites,
            ranges,
        }
    }
}
fn scope_ranges(model: &FileModel<'_>) -> Vec<Range> {
    model
        .scopes
        .iter()
        .map(|s| at(model.tree.root_node(), s.entered_at).map(|n| (n.start_byte(), n.end_byte())))
        .collect()
}
fn implicit_sites(model: &FileModel<'_>, ranges: &[Range], vcalls: &HashSet<usize>) -> Vec<Site> {
    descendants(model.tree.root_node())
        .filter(|n| implicit_node(model, *n, vcalls))
        .filter_map(|n| {
            scope_at(model, ranges, n.start_byte())
                .map(|scope| (n.start_byte(), scope, bare_super(n)))
        })
        .collect()
}
fn implicit_node(model: &FileModel<'_>, n: Node<'_>, vcalls: &HashSet<usize>) -> bool {
    bare_super(n) || bare_binding(model, n, vcalls) || binding_call(model, n)
}
fn bare_binding(model: &FileModel<'_>, n: Node<'_>, vcalls: &HashSet<usize>) -> bool {
    n.kind() == "identifier" && model.text(n) == "binding" && vcalls.contains(&n.start_byte())
}
fn implicit_read(model: &FileModel<'_>, byte: usize, vcalls: &HashSet<usize>) -> bool {
    at(model.tree.root_node(), byte).is_some_and(|n| {
        bare_super(n)
            || bare_binding(model, n, vcalls)
            || n.parent()
                .is_some_and(|p| p.start_byte() == byte && binding_call(model, p))
    })
}
fn explicit_reads(model: &FileModel<'_>, implicit: &HashSet<usize>) -> HashMap<String, Vec<usize>> {
    let mut explicit = HashMap::<String, Vec<usize>>::new();
    for scope in model.scopes.iter() {
        for (name, entry) in &scope.entries {
            explicit.entry(name.to_string()).or_default().extend(
                entry
                    .reads
                    .iter()
                    .filter(|r| !implicit.contains(&r.byte))
                    .map(|r| r.byte),
            );
        }
    }
    for bytes in explicit.values_mut() {
        bytes.sort_unstable();
        bytes.dedup();
    }
    explicit
}
fn scope_at(model: &FileModel<'_>, ranges: &[Range], byte: usize) -> Option<usize> {
    model
        .scopes
        .iter()
        .enumerate()
        .filter(|(i, s)| {
            s.entered_at <= byte
                && ranges[*i].is_some_and(|(start, end)| start <= byte && byte < end)
        })
        .max_by_key(|(_, s)| s.entered_at)
        .map(|(i, _)| i)
}
pub(in crate::cop::ported::difficult::shadow) fn reads(
    model: &FileModel<'_>,
    scope: usize,
    name: &str,
    entry: &Entry,
    index: &Index,
) -> Vec<Read> {
    let mut reads = entry
        .reads
        .iter()
        .filter(|r| !index.implicit.contains(&r.byte))
        .copied()
        .collect::<Vec<_>>();
    let Some(range) = index.ranges[scope] else {
        return reads;
    };
    reads.extend(
        index
            .sites
            .iter()
            .filter(|site| references_argument(model, **site, range, scope, name))
            .map(|site| Read {
                byte: site.0,
                under_defined: false,
            }),
    );
    reads
}
fn references_argument(
    model: &FileModel<'_>,
    (byte, scope, is_super): Site,
    (start, end): (usize, usize),
    owner: usize,
    name: &str,
) -> bool {
    if byte < start || byte >= end {
        return false;
    }
    if is_super {
        model.scopes[owner].kind == ScopeKind::Method
            && enclosing_method(model, scope) == Some(owner)
    } else {
        model.lookup(scope, byte, name) == Some(owner)
    }
}
fn binding_call(model: &FileModel<'_>, n: Node<'_>) -> bool {
    is_call(n)
        && field(n, "receiver").is_none()
        && field(n, "method").is_some_and(|m| model.text(m) == "binding")
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
