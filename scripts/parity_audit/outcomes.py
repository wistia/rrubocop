from .common import finding
from .policy import declared_source


def correction_result(case, mode, actual, expected):
    if actual.get("error"):
        return [finding(case, "native_error", "failure", mode=mode, actual=actual)]
    if actual.get("source") == expected["source"]:
        return []
    allowed = permitted(case, mode, actual, expected)
    return [finding(case, "correction", "exception" if allowed else "failure",
                    mode=mode, actual=actual, expected=expected)]


def syntax_results(case, mode, ruby, actual):
    results = upstream_syntax(case, mode, ruby)
    compiled = ruby["native_compile"][mode]
    if native_syntax_regression(ruby, mode, actual):
        results.append(finding(case, "native_invalid_ruby", "failure", mode=mode, error=compiled["error"]))
    return results


def upstream_syntax(case, mode, ruby):
    expected, results = ruby[mode], []
    if not expected["compile"]["valid"] and ruby["original_compile"]["valid"]:
        results.append(finding(case, "upstream_invalid_ruby", "upstream_hazard", mode=mode))
    if expected.get("error"):
        results.append(finding(case, "upstream_correction_cycle", "upstream_hazard", mode=mode))
    return results


def permitted(case, mode, actual, expected):
    guarded = not expected["compile"]["valid"] and actual.get("source") == case["source"]
    return declared_source(case, mode, actual.get("source")) or guarded


def native_syntax_regression(ruby, mode, actual):
    same_upstream = actual.get("source") == ruby[mode]["source"] and not ruby[mode]["compile"]["valid"]
    return not ruby["native_compile"][mode]["valid"] and ruby["original_compile"]["valid"] and not same_upstream
