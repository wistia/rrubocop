use crate::cop::CopRegistry;
use crate::testutil::run_cop_full;

fn count(cop: &str, source: &str) -> usize {
    run_cop_full(
        CopRegistry::default_registry().get(cop).unwrap(),
        source.as_bytes(),
    )
    .len()
}

#[test]
fn redis_transaction_delimiters_keep_the_same_guard() {
    for (open, close) in [("do", "end"), ("{", "}")] {
        for method in ["pipelined", "multi"] {
            let source = format!(
                "$redis.{method} {open} |pipe|\n pipe.set('a', 'v')\n pipe.set('b', 'v')\n{close}\n"
            );
            assert_eq!(count("Cops/NoSetWithoutExpiration", &source), 2, "{source}");
            let expired = source.replace("'v')", "'v', ex: 60)");
            assert_eq!(
                count("Cops/NoSetWithoutExpiration", &expired),
                0,
                "{expired}"
            );
        }
    }
}

#[test]
fn redis_transaction_receiver_does_not_leak_into_shadowed_blocks() {
    let source = "$redis.pipelined do |pipe|\n items.each do |pipe|\n  pipe.set('a', 'v')\n  pipe.set('b', 'v')\n end\n work\nend\n";
    assert_eq!(count("Cops/NoSetWithoutExpiration", source), 0);
}

#[test]
fn protected_write_comments_and_safe_navigation_keep_the_guard() {
    for receiver in ["record.", "record&."] {
        let protected = format!("{receiver}update_column(\n # reason\n :protected, true)\n");
        assert_eq!(
            count("Cops/NoCallbackSkippingProtectedWrites", &protected),
            1
        );
        let ordinary = protected.replace(":protected", ":name");
        assert_eq!(
            count("Cops/NoCallbackSkippingProtectedWrites", &ordinary),
            0
        );
    }
}

#[test]
fn anycable_only_checks_its_own_symbol_options() {
    for (source, expected) in [
        ("stream_from(\n # topic\n 'topic', coder: JSON)", 1),
        (
            "stream_from(\n # topic\n 'topic', coder: ActiveSupport:: JSON)",
            0,
        ),
        ("stream_for(record.with_options(coder: JSON))", 0),
        (
            "stream_for(record.with_options(coder: JSON), coder: JSON)",
            1,
        ),
        ("stream_for(record, {'coder' => JSON})", 0),
    ] {
        assert_eq!(count("AnyCable/StreamFrom", source), expected, "{source}");
    }
}

#[test]
fn rescue_reporting_uses_calls_and_constant_namespaces() {
    for (body, expected) in [
        ("raise", 0),
        ("raise = false; puts raise", 1),
        ("items.each { |raise| puts raise }", 1),
        ("raise = false; Kernel.raise(error)", 0),
        ("Rails.error().report(error)", 0),
        ("::Rails.error().report(error)", 0),
        ("Other::Rails.error().report(error)", 1),
        ("Sentry&.capture_exception(error)", 1),
    ] {
        let source = format!("begin\n work\nrescue\n {body}\nend\n");
        assert_eq!(
            count("Cops/RescueMustReportError", &source),
            expected,
            "{source}"
        );
    }
}

#[test]
fn redis_payload_keys_do_not_count_as_expiration_options() {
    for (source, expected) in [
        ("$redis.set('key', {ex: 60})", 1),
        ("$redis.set('key', {ex: 60}, ex: 60)", 0),
        ("$redis.set(\n # key\n 'key', {px: 60}, nx: true)", 1),
        (
            "$redis.set(\n # key\n 'key', {px: 60}, nx: true, ex: 60)",
            0,
        ),
    ] {
        assert_eq!(
            count("Cops/NoSetWithoutExpiration", source),
            expected,
            "{source}"
        );
    }
}

#[test]
fn shorthand_keywords_preserve_key_based_policies() {
    assert_eq!(
        count(
            "Cops/NoSetWithoutExpiration",
            "ex = 60; $redis.set('key', 'value', ex:)"
        ),
        0
    );
    assert_eq!(
        count(
            "Cops/NoCallbackSkippingProtectedWrites",
            "protected = true; record.update_columns(protected:)"
        ),
        1
    );
}

#[test]
fn transaction_comments_do_not_turn_one_statement_into_two() {
    let source = "redis.multi do |t|\n # comment\n t.set('a', 'v')\nend\n";
    assert_eq!(count("Cops/NoSetWithoutExpiration", source), 0);
    let repeated = source.replace(" t.set('a', 'v')", " t.set('a', 'v')\n t.set('b', 'v')");
    assert_eq!(count("Cops/NoSetWithoutExpiration", &repeated), 2);
}
#[test]
fn nested_reporters_keep_existing_descendant_semantics() {
    let source =
        "begin; work; rescue; begin; work; rescue; Sentry.capture_exception(error); end; end";
    assert_eq!(count("Cops/RescueMustReportError", source), 0);
    assert_eq!(
        count(
            "Cops/RescueMustReportError",
            &source.replace("Sentry.capture_exception(error)", "work")
        ),
        2
    );
}
