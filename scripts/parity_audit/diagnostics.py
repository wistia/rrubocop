from collections import Counter

from .common import finding
from .policy import declared_diagnostics, metadata, order, REPORT_ONLY


def native_diagnostics(result):
    return sorted([{
        "line": d["location"]["line"], "column": d["location"]["column"],
        "message": d["message"], "correctable": d["correctable"],
        "length": d["highlight_length"], "severity": d["severity"].lower(),
    } for d in result.get("diagnostics", [])], key=order)


def comparable_metadata(case, rows):
    values = metadata(rows)
    if case["cop"] in REPORT_ONLY:
        for row in values:
            row["correctable"] = False
    return values


def compare_diagnostics(case, native, ruby):
    actual = native_diagnostics(native)
    expected = sorted(ruby["diagnostics"], key=order)
    results = []
    if metadata(actual) != metadata(expected):
        allowed = allowed_metadata(case, actual, expected)
        results.append(finding(case, "diagnostics", "exception" if allowed else "failure",
                               actual=actual, expected=expected))
    results.extend(compare_ranges(case, actual, expected))
    return results


def range_groups(rows):
    groups = {}
    for row in rows:
        groups.setdefault(order(row), []).append((row["length"], row["severity"]))
    return {key: Counter(values) for key, values in groups.items()}


def compare_ranges(case, actual, expected):
    actual_groups, expected_groups = range_groups(actual), range_groups(expected)
    results = []
    for key in sorted(actual_groups.keys() & expected_groups.keys()):
        a, b = actual_groups[key], expected_groups[key]
        if a - b:
            results.append(range_finding(case, key, a, b))
    return results


def allowed_metadata(case, actual, expected):
    allowed = declared_diagnostics(case, actual)
    if case["cop"] in REPORT_ONLY:
        return allowed and comparable_metadata(case, actual) == comparable_metadata(case, expected)
    return allowed


def range_finding(case, key, actual, expected):
    return finding(case, "range_or_severity", "failure", location=key,
                   actual=sorted(actual.elements()), expected=sorted(expected.elements()))
