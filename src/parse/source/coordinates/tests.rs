use super::*;

fn old_offset(starts: &[usize], end: usize, column: usize) -> Option<usize> {
    if column == 0 {
        return Some(0);
    }
    starts
        .get(column)
        .copied()
        .or_else(|| (column == starts.len()).then_some(end))
}

fn compare(bytes: &[u8]) {
    let columns = Columns::new(bytes);
    let starts: Vec<_> = bytes
        .iter()
        .enumerate()
        .filter_map(|(index, byte)| (byte & 0xC0 != 0x80).then_some(index))
        .collect();
    for offset in 0..=bytes.len() {
        assert_eq!(
            columns.column_at(offset),
            starts.partition_point(|&start| start < offset),
            "{bytes:?} offset {offset}"
        );
    }
    for column in 0..=bytes.len() + 1 {
        assert_eq!(
            columns.offset_at(column),
            old_offset(&starts, bytes.len(), column),
            "{bytes:?} column {column}"
        );
    }
}

#[test]
fn sparse_and_dense_columns_match_original_arbitrary_byte_boundaries() {
    let mut seed = 37usize;
    for length in 0..128 {
        for _ in 0..32 {
            let bytes: Vec<_> = (0..length)
                .map(|_| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    (seed >> 16) as u8
                })
                .collect();
            compare(&bytes);
        }
    }
}

#[test]
fn continuation_runs_preserve_leading_trailing_and_mid_character_counts() {
    for bytes in [
        &b"ascii"[..],
        &b"\x80\x81abc"[..],
        &b"abc\x80\x81"[..],
        &b"abc\x80\x81def\x80\x81xyz"[..],
        &b"\x80\x81"[..],
        "ASCIIéASCII😀ASCIIβ".as_bytes(),
        "é😀β".as_bytes(),
    ] {
        compare(bytes);
    }
}

#[test]
fn mostly_ascii_lines_use_sparse_runs_and_unicode_heavy_lines_keep_dense_storage() {
    let text = format!("{}é", "x".repeat(1_000_000));
    let sparse = Columns::new(text.as_bytes());
    assert!(matches!(sparse, Columns::Sparse(_)));
    assert_eq!(sparse.offset_at(1_000_001), Some(1_000_002));
    assert_eq!(sparse.column_at(1_000_001), 1_000_001);
    assert!(matches!(
        Columns::new("é😀β".as_bytes()),
        Columns::Dense { .. }
    ));
}
