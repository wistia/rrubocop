use crate::parse::source::SourceFile;

struct LineCounts {
    code: Vec<usize>,
    comments: Vec<usize>,
}

pub(super) fn count(source: &SourceFile, start: usize, end: usize, comments: bool) -> usize {
    if start >= end {
        return 0;
    }
    let counts = source.cached(0, || {
        let mut code = Vec::with_capacity(source.line_count() + 1);
        let mut comments = Vec::with_capacity(source.line_count() + 1);
        code.push(0);
        comments.push(0);
        for line in source.lines() {
            let mut trimmed = line;
            while trimmed.first().is_some_and(|b| matches!(b, b' ' | b'\t')) {
                trimmed = &trimmed[1..];
            }
            while trimmed
                .last()
                .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r'))
            {
                trimmed = &trimmed[..trimmed.len() - 1];
            }
            let significant = !trimmed.is_empty();
            code.push(
                code.last().unwrap() + usize::from(significant && !trimmed.starts_with(b"#")),
            );
            comments.push(comments.last().unwrap() + usize::from(significant));
        }
        LineCounts { code, comments }
    });
    let prefix = if comments {
        &counts.comments
    } else {
        &counts.code
    };
    prefix[(end - 1).min(source.line_count())] - prefix[(start - 1).min(source.line_count())]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_counts_keep_comments_carriage_returns_and_final_empty_line() {
        let source = SourceFile::from_bytes(
            "counts.rb",
            b"class A\r\n # comment\r\n\t\r\n value\r\nend\n".to_vec(),
        );
        assert_eq!(count(&source, 2, 5, false), 1);
        assert_eq!(count(&source, 2, 5, true), 2);
        assert_eq!(count(&source, 1, 7, false), 3);
        assert_eq!(count(&source, 4, 4, true), 0);
    }
}
