use super::*;
use crate::diagnostic::{Location, Severity};

fn diagnostic(line: Arc<str>) -> Diagnostic {
    Diagnostic {
        path: "a.rb".into(),
        location: Location { line: 1, column: 1 },
        severity: Severity::Warning,
        cop_name: "Test/Cop".into(),
        message: "message".into(),
        corrected: true,
        correctable: true,
        source_line: line,
        highlight_length: 2,
    }
}

#[test]
fn line_table_roundtrip_shares_snapshots_without_changing_external_diagnostic_serde() {
    let original: Arc<str> = "é original".into();
    let diagnostics = vec![
        diagnostic(original.clone()),
        diagnostic(original),
        diagnostic("changed".into()),
    ];
    let bytes = encode(17, &diagnostics).unwrap();
    let encoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        encoded["source_lines"],
        serde_json::json!(["é original", "changed"])
    );
    assert!(encoded["diagnostics"][0].get("source_line").is_none());
    let restored = decode(&bytes).unwrap();
    assert!(Arc::ptr_eq(
        &restored[0].source_line,
        &restored[1].source_line
    ));
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        serde_json::to_value(&diagnostics).unwrap()
    );
    assert_eq!(
        restored[0].render(crate::formatter::color::Color::resolve(Some(false))),
        diagnostics[0].render(crate::formatter::color::Color::resolve(Some(false)))
    );
    assert_eq!(timestamp(&bytes), Some(17));
}

#[test]
fn encoded_payload_is_linear_in_shared_source_lines_not_offense_line_product() {
    let line: Arc<str> = "x".repeat(10000).into();
    let diagnostics = vec![diagnostic(line); 256];
    let bytes = encode(1, &diagnostics).unwrap();
    assert!(bytes.len() < 100000, "{} bytes", bytes.len());
    assert!(bytes.len() * 16 < legacy_bytes(&diagnostics).len());
    assert_eq!(decode(&bytes).unwrap().len(), 256);
}

#[test]
fn malformed_line_tables_are_misses_and_legacy_payloads_remain_readable() {
    let diagnostics = vec![diagnostic("old source".into())];
    let legacy =
        serde_json::to_vec(&serde_json::json!({"ts": 1, "diagnostics": diagnostics})).unwrap();
    assert_eq!(
        decode(&legacy).unwrap()[0].source_line.as_ref(),
        "old source"
    );
    malformed_tables(&diagnostics);
}
fn malformed_tables(diagnostics: &[Diagnostic]) {
    let mut encoded: serde_json::Value =
        serde_json::from_slice(&encode(2, diagnostics).unwrap()).unwrap();
    encoded["line_indices"] = serde_json::json!([99]);
    assert!(decode(&serde_json::to_vec(&encoded).unwrap()).is_none());
    encoded["line_indices"] = serde_json::json!([]);
    assert!(decode(&serde_json::to_vec(&encoded).unwrap()).is_none());
    encoded.as_object_mut().unwrap().remove("line_indices");
    assert!(decode(&serde_json::to_vec(&encoded).unwrap()).is_none());
    assert!(decode(b"not json").is_none());
}

#[test]
fn malformed_schema_and_missing_timestamp_are_cache_misses() {
    assert!(decode(b"{\"diagnostics\":[]}").is_none());
    assert!(decode(b"{\"format\":2,\"ts\":1,\"diagnostics\":[]}").is_none());
    assert!(
        decode(
            b"{\"format\":99,\"ts\":1,\"diagnostics\":[],\"source_lines\":[],\"line_indices\":[]}"
        )
        .is_none()
    );
    assert_eq!(timestamp(b"{\"ts\":7,\"diagnostics\":42}"), Some(7));
}

fn legacy_bytes(diagnostics: &[Diagnostic]) -> Vec<u8> {
    #[derive(serde::Serialize)]
    struct Legacy<'a> {
        ts: u64,
        diagnostics: &'a [Diagnostic],
    }
    serde_json::to_vec(&Legacy { ts: 1, diagnostics }).unwrap()
}
