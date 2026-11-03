use tree_sitter::Node;

use super::named_kids;
use crate::parse::source::SourceFile;

pub(crate) fn heredoc_interpolations<'a>(
    source: &SourceFile,
    node: Node<'a>,
) -> Option<Vec<Node<'a>>> {
    let (start, end) = crate::cop::ported::heredoc_span(source, node)?;
    let root = std::iter::successors(Some(node), |node| node.parent()).last()?;
    let mut pending = vec![root];
    let mut body = None;
    while let Some(node) = pending.pop() {
        if node.kind() == "heredoc_body"
            && node.start_byte() <= start
            && node.end_byte() >= end
            && body.is_none_or(|body: Node<'a>| node.byte_range().len() < body.byte_range().len())
        {
            body = Some(node);
        }
        pending.extend(named_kids(node));
    }
    let mut pending = named_kids(body?);
    let mut result = Vec::new();
    while let Some(node) = pending.pop() {
        if node.kind() == "heredoc_body" {
            continue;
        }
        if node.kind() == "interpolation" {
            result.push(node);
        } else {
            pending.extend(named_kids(node));
        }
    }
    result.sort_by_key(Node::start_byte);
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_heredoc_interpolation_membership_and_ranges() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/core-heredoc-interpolations.json"
        )))
        .unwrap();
        for case in cases.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                case["source"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(!tree.root_node().has_error(), "{}", case["id"]);
            let mut pending = vec![tree.root_node()];
            let mut openers = Vec::new();
            while let Some(node) = pending.pop() {
                if node.kind() == "heredoc_beginning" {
                    openers.push(node);
                }
                pending.extend(named_kids(node));
            }
            openers.sort_by_key(Node::start_byte);
            let observed:Vec<_>=openers.into_iter().map(|opener| {
                let (line,column)=source.offset_to_line_col(opener.start_byte());
                let interpolations:Vec<_>=heredoc_interpolations(&source,opener).unwrap().into_iter().map(|interpolation| {
                    named_kids(interpolation).into_iter().filter(|node|!matches!(node.kind(),"comment"|"empty_statement")).map(|expression| {
                        let (line,column)=source.offset_to_line_col(expression.start_byte());
                        serde_json::json!({"source":String::from_utf8_lossy(&source.as_bytes()[expression.byte_range()]).replace("\r\n","\n"),"line":line,"column":column})
                    }).collect::<Vec<_>>()
                }).collect();
                serde_json::json!({"opener":String::from_utf8_lossy(&source.as_bytes()[opener.byte_range()]),"line":line,"column":column,"interpolations":interpolations})
            }).collect();
            assert_eq!(
                serde_json::json!(observed),
                case["heredocs"],
                "{}",
                case["id"]
            );
        }
    }
}
