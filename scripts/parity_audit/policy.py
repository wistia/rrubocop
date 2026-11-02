REPORT_ONLY = {"Lint/UselessTimes", "Style/MixinGrouping", "Style/MapCompactWithConditionalBlock"}


def metadata(rows):
    keys = ("line", "column", "message", "correctable")
    return sorted([{k: d[k] for k in keys} for d in rows], key=lambda d: (order(d), d["correctable"]))


def order(diagnostic):
    return (diagnostic["line"], diagnostic["column"], diagnostic["message"])


def declared_diagnostics(case, actual):
    if case["cop"] in REPORT_ONLY:
        return all(not d["correctable"] for d in actual)
    if not case.get("reason") or not {"rust_expected", "rust_correctable"}.intersection(case):
        return False
    return declared_metadata(case) == metadata(actual)


def declared_source(case, mode, actual):
    if case["cop"] in REPORT_ONLY:
        return actual == case["source"]
    if not case.get("reason") or "rust_corrected" not in case:
        return False
    if mode == "safe" and not safe_correction(case):
        return actual == case["source"]
    return actual == case.get("rust_corrected", case["corrected"])


def declared_metadata(case):
    wanted = metadata(case.get("rust_expected", case["expected"]))
    for row in wanted:
        row["correctable"] = case.get("rust_correctable", row["correctable"])
    return wanted


def safe_correction(case):
    options = case["config"]
    return options.get("Safe", True) and options.get("SafeAutoCorrect", True)
