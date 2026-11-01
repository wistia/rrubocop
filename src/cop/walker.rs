//! Walk tree-sitter AST and dispatch to cops by interested node kinds.

use std::collections::HashMap;

use tree_sitter::Node;
mod parents;
use parents::Parents;

use crate::cop::shared::call_method_name;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

fn is_call_kind(kind: &str) -> bool {
    matches!(kind, "call" | "command" | "command_call")
}

pub struct BatchedWalker<'a> {
    cops: Vec<&'a dyn Cop>,
    configs: Vec<&'a CopConfig>,
    /// Per-cop: pass a corrections buffer (`-a`/`-A` gating).
    corr_ok: Vec<bool>,
    /// Non-call kinds → cop indices.
    kinds: HashMap<&'static str, Vec<usize>>,
    /// Call-like cops with empty `interested_call_names` (every call/command).
    call_all: Vec<usize>,
    /// Call-like cops gated by method name.
    call_by_name: HashMap<&'static [u8], Vec<usize>>,
    identifier_by_name: HashMap<&'static [u8], Vec<usize>>,
    parents: Parents,
}

impl<'a> BatchedWalker<'a> {
    pub fn new(cops: Vec<&'a dyn Cop>, configs: Vec<&'a CopConfig>) -> Self {
        let corr_ok = vec![true; cops.len()];
        Self::with_corr_ok(cops, configs, corr_ok)
    }

    pub fn with_corr_ok(
        cops: Vec<&'a dyn Cop>,
        configs: Vec<&'a CopConfig>,
        corr_ok: Vec<bool>,
    ) -> Self {
        debug_assert_eq!(cops.len(), configs.len());
        debug_assert_eq!(cops.len(), corr_ok.len());
        let mut kinds: HashMap<&'static str, Vec<usize>> = HashMap::new();
        let mut call_all = Vec::new();
        let mut call_by_name: HashMap<&'static [u8], Vec<usize>> = HashMap::new();
        let mut identifier_by_name = HashMap::new();
        let mut parents = Parents::default();
        for (i, cop) in cops.iter().enumerate() {
            register_cop(
                i,
                *cop,
                &mut kinds,
                &mut call_all,
                &mut call_by_name,
                &mut identifier_by_name,
                &mut parents,
            );
        }
        merge_identifier_cops(&kinds, &mut identifier_by_name);
        parents.merge(&kinds);
        Self {
            cops,
            configs,
            corr_ok,
            kinds,
            call_all,
            call_by_name,
            identifier_by_name,
            parents,
        }
    }

    pub fn walk(
        &self,
        source: &SourceFile,
        root: Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        self.visit(
            source,
            root,
            root.parent().map(|n| n.kind()),
            diagnostics,
            &mut corrections,
        );
    }

    fn visit(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        parent: Option<&str>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<Correction>>,
    ) {
        // Dispatch only on named nodes. Anonymous keyword/punct tokens share kinds
        // like `rescue`/`ensure`/`else` with real clauses and would false-positive.
        if node.is_named() {
            self.dispatch(source, node, parent, diagnostics, corrections);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit(source, child, Some(node.kind()), diagnostics, corrections);
        }
    }

    fn dispatch(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        parent: Option<&str>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<Correction>>,
    ) {
        let kind = node.kind();
        if is_call_kind(kind) {
            self.dispatch_call(source, node, diagnostics, corrections);
            return;
        }
        if let Some(idxs) = self.node_cops(source, node, parent) {
            for &i in idxs {
                self.run_cop(i, source, node, diagnostics, corrections);
            }
        }
    }

    fn node_cops(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        parent: Option<&str>,
    ) -> Option<&Vec<usize>> {
        if node.kind() == "identifier" {
            let name = &source.as_bytes()[node.byte_range()];
            if let Some(indices) = self.identifier_by_name.get(name) {
                return Some(indices);
            }
        }
        self.parents
            .get(node.kind(), parent)
            .or_else(|| self.kinds.get(node.kind()))
    }

    fn dispatch_call(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<Correction>>,
    ) {
        for &i in &self.call_all {
            self.run_cop(i, source, node, diagnostics, corrections);
        }
        if let Some(name) = call_method_name(source, node) {
            if let Some(idxs) = self.call_by_name.get(name) {
                for &i in idxs {
                    self.run_cop(i, source, node, diagnostics, corrections);
                }
            }
        }
    }

    fn run_cop(
        &self,
        i: usize,
        source: &SourceFile,
        node: Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<Correction>>,
    ) {
        let diag_before = diagnostics.len();
        let edit_before = corrections.as_ref().map_or(0, |edits| edits.len());
        let corr = if self.corr_ok[i] {
            corrections.as_deref_mut()
        } else {
            None
        };
        self.cops[i].check_node(source, node, self.configs[i], diagnostics, corr);
        if let Some(edits) = corrections.as_deref_mut() {
            crate::correction::assign_owners(
                source,
                &diagnostics[diag_before..],
                &mut edits[edit_before..],
            );
        }
    }
}

fn register_cop(
    i: usize,
    cop: &dyn Cop,
    kinds: &mut HashMap<&'static str, Vec<usize>>,
    call_all: &mut Vec<usize>,
    call_by_name: &mut HashMap<&'static [u8], Vec<usize>>,
    identifier_by_name: &mut HashMap<&'static [u8], Vec<usize>>,
    parents: &mut Parents,
) {
    // Empty interested_node_kinds => not an AST cop (skipped).
    let mut saw_call = false;
    for &k in cop.interested_node_kinds() {
        if is_call_kind(k) {
            if saw_call {
                continue;
            }
            saw_call = true;
            let names = cop.interested_call_names();
            if names.is_empty() {
                call_all.push(i);
            } else {
                for &n in names {
                    call_by_name.entry(n).or_default().push(i);
                }
            }
        } else {
            register_node(i, cop, k, kinds, identifier_by_name, parents);
        }
    }
}

fn register_node(
    i: usize,
    cop: &dyn Cop,
    kind: &'static str,
    kinds: &mut HashMap<&'static str, Vec<usize>>,
    identifiers: &mut HashMap<&'static [u8], Vec<usize>>,
    parents: &mut Parents,
) {
    if !register_identifier(i, cop, kind, identifiers) && !parents.register(i, cop, kind) {
        kinds.entry(kind).or_default().push(i);
    }
}

fn register_identifier(
    i: usize,
    cop: &dyn Cop,
    kind: &str,
    by_name: &mut HashMap<&'static [u8], Vec<usize>>,
) -> bool {
    let names = cop.interested_identifier_names();
    if kind != "identifier" || names.is_empty() {
        return false;
    }
    for name in names {
        by_name.entry(name).or_default().push(i);
    }
    true
}

fn merge_identifier_cops(
    kinds: &HashMap<&'static str, Vec<usize>>,
    by_name: &mut HashMap<&'static [u8], Vec<usize>>,
) {
    if let Some(ungated) = kinds.get("identifier") {
        for indices in by_name.values_mut() {
            indices.extend(ungated);
            indices.sort_unstable();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingCop {
        kinds: &'static [&'static str],
        names: &'static [&'static [u8]],
        hits: AtomicUsize,
    }

    impl Cop for CountingCop {
        fn name(&self) -> &'static str {
            "Test/Counting"
        }
        fn interested_node_kinds(&self) -> &'static [&'static str] {
            self.kinds
        }
        fn interested_call_names(&self) -> &'static [&'static [u8]] {
            self.names
        }
        fn check_node(
            &self,
            _source: &SourceFile,
            _node: Node<'_>,
            _config: &CopConfig,
            _diagnostics: &mut Vec<Diagnostic>,
            _corrections: Option<&mut Vec<Correction>>,
        ) {
            self.hits.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn counting(names: &'static [&'static [u8]]) -> CountingCop {
        CountingCop {
            kinds: &["call", "command"],
            names,
            hits: AtomicUsize::new(0),
        }
    }

    fn hit_count(cop: &CountingCop) -> usize {
        cop.hits.load(Ordering::Relaxed)
    }

    fn walk_sample(gated: &CountingCop, ungated: &CountingCop) {
        let cfg = CopConfig::default();
        let walker = BatchedWalker::new(vec![gated, ungated], vec![&cfg, &cfg]);
        let sf = SourceFile::from_bytes(PathBuf::from("t.rb"), b"foo.each {}; bar.map {}".to_vec());
        let tree = crate::parse::parse_ruby(&sf).unwrap();
        assert!(!tree.root_node().has_error());
        walker.walk(&sf, tree.root_node(), &mut Vec::new(), None);
    }

    struct CorrCop;

    impl Cop for CorrCop {
        fn name(&self) -> &'static str {
            "Test/Corr"
        }
        fn interested_node_kinds(&self) -> &'static [&'static str] {
            &["call"]
        }
        fn check_node(
            &self,
            _source: &SourceFile,
            node: Node<'_>,
            _config: &CopConfig,
            _diagnostics: &mut Vec<Diagnostic>,
            corrections: Option<&mut Vec<Correction>>,
        ) {
            if let Some(corr) = corrections {
                corr.push(Correction {
                    start: node.start_byte(),
                    end: node.end_byte(),
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                    owner: None,
                });
            }
        }
    }

    fn walk_with_corr(cop: &CorrCop, corr_ok: bool, corr: &mut Vec<Correction>) {
        let cfg = CopConfig::default();
        let sf = SourceFile::from_bytes(PathBuf::from("t.rb"), b"foo()\n".to_vec());
        let tree = crate::parse::parse_ruby(&sf).unwrap();
        assert!(!tree.root_node().has_error());
        BatchedWalker::with_corr_ok(vec![cop], vec![&cfg], vec![corr_ok]).walk(
            &sf,
            tree.root_node(),
            &mut Vec::new(),
            Some(corr),
        );
    }

    #[test]
    fn call_name_gate_skips_non_matching_methods() {
        let gated = counting(&[b"map"]);
        let ungated = counting(&[]);
        walk_sample(&gated, &ungated);
        assert_eq!(hit_count(&gated), 1);
        assert_eq!(hit_count(&ungated), 2);
    }

    #[test]
    fn corr_ok_false_does_not_collect_corrections() {
        let cop = CorrCop;
        let mut corr = Vec::new();
        walk_with_corr(&cop, false, &mut corr);
        assert!(corr.is_empty(), "disallowed cops must not push corrections");
        walk_with_corr(&cop, true, &mut corr);
        assert!(!corr.is_empty());
    }
}

#[cfg(test)]
mod identifier_tests;
