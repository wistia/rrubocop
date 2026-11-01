use super::*;
use std::sync::Mutex;

struct IdentifierCop<'a> {
    label: &'static str,
    names: &'static [&'static [u8]],
    hits: &'a Mutex<Vec<&'static str>>,
}
impl Cop for IdentifierCop<'_> {
    fn name(&self) -> &'static str {
        self.label
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["identifier", "integer"]
    }
    fn interested_identifier_names(&self) -> &'static [&'static [u8]] {
        self.names
    }
    fn check_node(
        &self,
        _: &SourceFile,
        _: Node<'_>,
        _: &CopConfig,
        _: &mut Vec<Diagnostic>,
        _: Option<&mut Vec<Correction>>,
    ) {
        self.hits.lock().unwrap().push(self.label);
    }
}
fn inspect(code: &str) -> Vec<&'static str> {
    let hits = Mutex::new(Vec::new());
    let first = IdentifierCop {
        label: "First",
        names: &[b"sleep"],
        hits: &hits,
    };
    let all = IdentifierCop {
        label: "All",
        names: &[],
        hits: &hits,
    };
    let last = IdentifierCop {
        label: "Last",
        names: &[b"sleep"],
        hits: &hits,
    };
    inspect_cops(code, &[&first, &all, &last]);
    hits.into_inner().unwrap()
}
fn inspect_cops(code: &str, cops: &[&dyn Cop]) {
    let source = SourceFile::from_bytes("test.rb", code.as_bytes().to_vec());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let config = CopConfig::default();
    BatchedWalker::new(cops.to_vec(), vec![&config; cops.len()]).walk(
        &source,
        tree.root_node(),
        &mut vec![],
        None,
    );
}
#[test]
fn identifier_gates_skip_irrelevant_names_and_preserve_registration_order() {
    assert_eq!(
        inspect("ordinary\nsleep\nother\n"),
        ["All", "First", "All", "Last", "All"]
    );
}
#[test]
fn identifier_gates_do_not_filter_other_node_kinds() {
    assert_eq!(inspect("42\n"), ["First", "All", "Last"]);
}
