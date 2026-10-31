use super::super::context::*;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::{codemap::CodeMap, source::SourceFile};
use tree_sitter::Node;
pub(super) struct OutOfRangeRegexpRef;
impl Cop for OutOfRangeRegexpRef {
    fn name(&self) -> &'static str {
        "Lint/OutOfRangeRegexpRef"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn uses_source_phase(&self) -> bool {
        true
    }
    fn check_source(
        &self,
        source: &SourceFile,
        tree: &tree_sitter::Tree,
        _map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !has_reference(source) {
            return;
        }
        let c = Context::new(source, config);
        let mut state = State {
            c: &c,
            valid: Some(0),
            findings: vec![],
            heredocs: heredoc::Heredocs::new(&c, tree.root_node()),
        };
        state.walk(tree.root_node());
        for f in state.findings {
            let (line, col) = source.offset_to_line_col(f.start);
            let mut d = self.diagnostic(source, line, col, f.message);
            d.highlight_length = f.end - f.start;
            diagnostics.push(d);
        }
    }
}
struct State<'a, 's, 'n> {
    c: &'a Context<'s>,
    valid: Option<usize>,
    findings: Vec<Finding>,
    heredocs: heredoc::Heredocs<'n>,
}
mod heredoc;
mod walk;
impl State<'_, '_, '_> {
    fn count(&self, n: Node<'_>) -> Option<usize> {
        if n.kind() != "regex" {
            return None;
        }
        super::super::style_regexp::capture_counts(self.c, n)
            .map(|(named, numbered)| if named > 0 { named } else { numbered })
    }
    fn maximum<'n>(&self, nodes: impl IntoIterator<Item = Node<'n>>) -> Option<usize> {
        nodes.into_iter().filter_map(|n| self.count(n)).max()
    }
    fn reference(&mut self, n: Node<'_>) {
        let Some(digits) = reference_digits(self.c.text(n)) else {
            return;
        };
        let Some(count) = self.valid else {
            return;
        };
        if digits.parse::<usize>().is_ok_and(|number| number <= count) {
            return;
        }
        self.findings.push(Finding::new(n, message(digits, count)));
    }
}
fn argument_method(method: &str) -> bool {
    matches!(
        method,
        "=~" | "match"
            | "grep"
            | "gsub"
            | "gsub!"
            | "sub"
            | "sub!"
            | "[]"
            | "slice"
            | "slice!"
            | "index"
            | "rindex"
            | "scan"
            | "partition"
            | "rpartition"
            | "start_with?"
            | "end_with?"
    )
}
fn restricted(method: &str) -> bool {
    argument_method(method) || method == "==="
}

fn reference_digits(text: &str) -> Option<&str> {
    text.strip_prefix('$').filter(|s| {
        s.starts_with(|c: char| ('1'..='9').contains(&c)) && s.bytes().all(|b| b.is_ascii_digit())
    })
}
fn message(digits: &str, count: usize) -> String {
    let amount = if count == 0 {
        "no".into()
    } else {
        count.to_string()
    };
    let group = if count == 1 { "group" } else { "groups" };
    format!("${digits} is out of range ({amount} regexp capture {group} detected).")
}

fn has_reference(source: &SourceFile) -> bool {
    source
        .as_bytes()
        .windows(2)
        .any(|pair| pair[0] == b'$' && (b'1'..=b'9').contains(&pair[1]))
}
