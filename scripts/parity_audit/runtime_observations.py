from .runtime_activity_counts import MODES


def runtime_records(cases, records):
    wanted = {(case["id"], mode) for case in cases for mode in MODES}
    indexed = {}
    for record in records:
        key = (record["id"], record["mode"])
        if key not in wanted or key in indexed:
            raise ValueError("Unexpected or duplicate runtime result IDs")
        if any(not isinstance(record.get(name), dict) for name in ("original", "native", "ruby")):
            raise ValueError("Incomplete runtime observation")
        indexed[key] = record
    return indexed


def measure_case(case, mode, native, reference, records, status):
    result = native.get((case["id"], mode), {})
    record = records.get((case["id"], mode))
    row = observation(case, mode, result, record)
    blocked = blocking_reason(case, mode, native, reference, record, status)
    if blocked:
        return {**row, "activity": "blocked", "blocked_reason": blocked}
    if row["source_changed"]:
        return {**row, "activity": "positive_rewrites"}
    return {**row, "activity": unchanged_activity(native.get((case["id"], "off"), {}))}


def observation(case, mode, result, record):
    source = result.get("source")
    return {"id": case["id"], "cop": case["cop"], "mode": mode,
            "source_changed": isinstance(source, str) and source != case["source"],
            "behavior": behavior(record)}


def blocking_reason(case, mode, native, reference, record, status):
    if status in ("skipped", "unavailable"):
        return status
    result = native.get((case["id"], mode), {})
    compilation = compile_block(case, mode, native, reference)
    if compilation:
        return compilation
    execution = execution_block(record)
    if execution:
        return execution
    if not isinstance(result.get("source"), str):
        return "missing_native_source"
    return None


def compile_block(case, mode, native, reference):
    key = case["id"]
    if native.get((key, mode), {}).get("error") or native.get((key, "off"), {}).get("error"):
        return "native_error"
    return compilation_status(reference.get(key, {}), mode)


def compilation_status(ruby, mode):
    if not ruby.get("original_compile", {}).get("valid"):
        return "original_invalid_or_unverified"
    if not ruby.get("native_compile", {}).get(mode, {}).get("valid"):
        return "native_invalid_or_unverified"
    return None


def execution_block(record):
    if record is None:
        return "missing_execution"
    if any("runner_error" in record[name] for name in ("original", "native", "ruby")):
        return "runner_error"
    if "exception" in record["original"]:
        return "original_exception"
    if "exception" in record["native"]:
        return "native_exception"
    if behavior(record) == "native_failure":
        return "native_behavior_failure"
    return None


def unchanged_activity(original):
    diagnostics = original.get("diagnostics", [])
    if not diagnostics:
        return "unchanged_negative"
    if all(not diagnostic.get("correctable") for diagnostic in diagnostics):
        return "unchanged_report_only"
    return "unchanged_not_applied"


def behavior(record):
    if record is None:
        return "unobserved"
    original, native, ruby = (record[name] for name in ("original", "native", "ruby"))
    if any("runner_error" in result for result in (original, native, ruby)):
        return "runner_failure"
    if native != original:
        return "shared_upstream_change" if native == ruby else "native_failure"
    return "preserved_despite_reference_change" if ruby != original else "preserved"

