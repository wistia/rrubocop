use super::{Directives, MARKER, apply_directive};
use crate::parse::{self, source::SourceFile};
use std::sync::Arc;

pub fn parse(src: &str) -> Directives {
    let source = SourceFile::from_bytes("directives.rb", src.as_bytes().to_vec());
    let tree = parse::parse_ruby(&source).expect("Ruby directive source parses");
    from_tree(&source, &tree)
}

pub(crate) fn for_source(source: &SourceFile, tree: &tree_sitter::Tree) -> Arc<Directives> {
    source.cached(0, || from_tree(source, tree))
}

pub(crate) fn cached(source: &SourceFile) -> Arc<Directives> {
    source.cached(0, || {
        let tree = parse::parse_ruby(source).expect("Ruby directive source parses");
        from_tree(source, &tree)
    })
}

fn from_tree(source: &SourceFile, tree: &tree_sitter::Tree) -> Directives {
    let mut directives = Directives::default();
    if !source
        .as_bytes()
        .windows(7)
        .any(|bytes| bytes.eq_ignore_ascii_case(b"rubocop"))
    {
        return directives;
    }
    let mut comments = Vec::new();
    collect_comments(tree.root_node(), &mut comments);
    comments.sort_by_key(tree_sitter::Node::start_byte);
    for comment in comments {
        apply_comment(&mut directives, source, comment);
    }
    directives
}

fn apply_comment(directives: &mut Directives, source: &SourceFile, comment: tree_sitter::Node<'_>) {
    let text = String::from_utf8_lossy(&source.as_bytes()[comment.byte_range()]);
    if !text.starts_with('#') {
        return;
    }
    let Some(marker) = MARKER.find(&text) else {
        return;
    };
    let (line_no, _) = source.offset_to_line_col(comment.start_byte());
    directives.comment_lines.push(line_no);
    directives.comment_starts.push(comment.start_byte());
    let line = if marker.start() > 0 || trailing(source, comment.start_byte(), line_no) {
        "code #"
    } else {
        "#"
    };
    apply_text(directives, &text[marker.end()..], line, line_no);
}

fn apply_text(directives: &mut Directives, text: &str, line: &str, line_no: usize) {
    for directive in MARKER.split(text) {
        apply_directive(directives, directive.trim(), line, line_no);
    }
}

fn trailing(source: &SourceFile, offset: usize, line: usize) -> bool {
    let start = source.line_start(line).unwrap();
    source.as_bytes()[start..offset]
        .iter()
        .any(|byte| !byte.is_ascii_whitespace())
}

fn collect_comments<'tree>(
    node: tree_sitter::Node<'tree>,
    comments: &mut Vec<tree_sitter::Node<'tree>>,
) {
    if node.kind() == "comment" {
        comments.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_comments(child, comments);
    }
}
