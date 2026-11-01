use super::*;
use std::sync::Mutex;

struct ParentCop<'a> {
    label: &'static str,
    gated: bool,
    hits: &'a Mutex<Vec<(&'static str, usize)>>,
}
impl Cop for ParentCop<'_> {
    fn name(&self) -> &'static str {
        self.label
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["constant", "scope_resolution", "integer"]
    }
    fn interested_parent_node_kinds(&self, kind: &str) -> &'static [&'static str] {
        if self.gated && matches!(kind, "constant" | "scope_resolution") {
            &["left_assignment_list", "destructured_left_assignment"]
        } else {
            &[]
        }
    }
    fn check_node(
        &self,
        _: &SourceFile,
        n: Node<'_>,
        _: &CopConfig,
        _: &mut Vec<Diagnostic>,
        _: Option<&mut Vec<Correction>>,
    ) {
        self.hits.lock().unwrap().push((self.label, n.start_byte()));
    }
}
fn inspect(text: &str, subtree: bool) -> Vec<(&'static str, usize)> {
    let hits = Mutex::new(Vec::new());
    let first = ParentCop {
        label: "First",
        gated: true,
        hits: &hits,
    };
    let all = ParentCop {
        label: "All",
        gated: false,
        hits: &hits,
    };
    let last = ParentCop {
        label: "Last",
        gated: true,
        hits: &hits,
    };
    inspect_cops(text, &[&first, &all, &last], subtree);
    hits.into_inner().unwrap()
}
fn inspect_cops(text: &str, cops: &[&dyn Cop], subtree: bool) {
    let source = SourceFile::from_bytes("parents.rb", text.as_bytes().to_vec());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let node = walk_root(tree.root_node(), subtree);
    let config = CopConfig::default();
    BatchedWalker::new(cops.to_vec(), vec![&config; cops.len()]).walk(
        &source,
        node,
        &mut vec![],
        None,
    );
}
fn walk_root(root: Node<'_>, subtree: bool) -> Node<'_> {
    if !subtree {
        return root;
    }
    root.named_child(0)
        .unwrap()
        .child_by_field_name("left")
        .unwrap()
        .named_child(0)
        .unwrap()
}
#[test]
fn parent_gates_preserve_each_target_and_interleaved_cop_order() {
    assert_eq!(
        inspect("Ref\nA, (B, C), D = values\n", false),
        [
            ("All", 0),
            ("First", 4),
            ("All", 4),
            ("Last", 4),
            ("First", 8),
            ("All", 8),
            ("Last", 8),
            ("First", 11),
            ("All", 11),
            ("Last", 11),
            ("First", 15),
            ("All", 15),
            ("Last", 15),
        ]
    );
}
#[test]
fn parent_gates_do_not_filter_other_node_kinds() {
    assert_eq!(
        inspect("42\n", false),
        [("First", 0), ("All", 0), ("Last", 0)]
    );
}
#[test]
fn a_subtree_walk_retains_its_real_parent_for_gating() {
    assert_eq!(
        inspect("A, B = values\n", true),
        [("First", 0), ("All", 0), ("Last", 0)]
    );
}

#[test]
fn scoped_targets_do_not_dispatch_their_reference_children_to_gated_cops() {
    assert_eq!(
        inspect("A::B, C::D = values\n", false),
        [
            ("First", 0),
            ("All", 0),
            ("Last", 0),
            ("All", 0),
            ("All", 3),
            ("First", 6),
            ("All", 6),
            ("Last", 6),
            ("All", 6),
            ("All", 9),
        ]
    );
}
