use super::*;

#[test]
fn display_columns_round_trip_unicode_and_line_boundaries() {
    let source = SourceFile::from_bytes("a.rb", "é😀x\nβ\n".as_bytes().to_vec());
    for offset in [0, 2, 6, 7, 8, 10, 11] {
        let (line, column) = source.offset_to_line_col(offset);
        assert_eq!(source.line_col_to_offset(line, column), Some(offset));
    }
    assert_eq!(source.line_col_to_offset(1, 99), None);
    assert_eq!(source.line_col_to_offset(99, 0), None);
}

#[test]
fn invalid_bytes_keep_the_existing_codepoint_counting_convention() {
    let source = SourceFile::from_bytes("a.rb", vec![0xff, b'x', b'\n']);
    for offset in 0..=3 {
        let (line, column) = source.offset_to_line_col(offset);
        assert_eq!(source.line_col_to_offset(line, column), Some(offset));
    }
}

#[test]
fn cached_unicode_columns_match_byte_prefix_counting_even_inside_characters() {
    let bytes = "é😀x\nASCII\nβeta".as_bytes().to_vec();
    let source = SourceFile::from_bytes("a.rb", bytes.clone());
    for offset in 0..=bytes.len() {
        let line = source.line_starts.partition_point(|&start| start <= offset);
        let start = source.line_start(line).unwrap();
        assert_eq!(
            source.offset_to_line_col(offset),
            (line, utf8_byte_index_to_column(&bytes[start..offset]))
        );
    }
    assert!(std::sync::Arc::ptr_eq(
        &source.columns(1),
        &source.columns(1)
    ));
    assert!(std::sync::Arc::ptr_eq(
        &source.columns(2),
        &source.columns(2)
    ));
}

#[test]
fn indexed_raw_lines_preserve_split_semantics() {
    for bytes in [
        b"".as_slice(),
        b"one",
        b"a\nb\n",
        b"a\r\n\r\n",
        b"\xff\n\xc3\xa9\r\n",
    ] {
        let source = super::SourceFile::from_bytes("raw.rb", bytes.to_vec());
        let expected: Vec<_> = source.lines().collect();
        assert_eq!(source.line_bytes(0), None);
        for (index, line) in expected.iter().enumerate() {
            assert_eq!(source.line_bytes(index + 1), Some(*line));
        }
        assert_eq!(source.line_bytes(expected.len() + 1), None);
    }
}
