from .common import finding, read_results
from .diagnostics import compare_diagnostics
from .outcomes import correction_result, syntax_results, manual_reference_guard


def validate_results(cases, native, ruby):
    wanted = {(c["id"], mode) for c in cases for mode in ("off", "safe", "all")}
    if set(native) != wanted or set(ruby) != {c["id"] for c in cases}:
        raise ValueError("Incomplete engine output; audit cannot claim coverage")


def index_results(rows, key):
    result = {key(row): row["result"] for row in rows}
    if len(result) != len(rows):
        raise ValueError("Duplicate engine result IDs")
    return result


def load(cases, output):
    native = index_results(read_results(output / "native.jsonl"), lambda r: (r["id"], r["mode"]))
    ruby = index_results(read_results(output / "reference.jsonl"), lambda r: r["id"])
    validate_results(cases, native, ruby)
    return native, ruby


def compare(cases, output):
    native, ruby = load(cases, output)
    results = []
    for case in cases:
        results.extend(Comparison(case, native, ruby[case["id"]]).run())
    return results


class Comparison:
    def __init__(self, case, native, ruby):
        self.case, self.native, self.ruby = case, native, ruby

    def run(self):
        return self.native_errors() + self.reference_results()

    def reference_results(self):
        if self.ruby.get("original_invalid"):
            return self.invalid_original()
        if "error" in self.ruby:
            return self.reference_error()
        results = self.diagnostics()
        for mode in ("safe", "all"):
            results.extend(self.correction(mode))
        return results

    def native_errors(self):
        results = []
        for mode in ("off", "safe", "all"):
            result = self.native[(self.case["id"], mode)]
            if result.get("error"):
                results.append(finding(self.case, "native_error", "failure", mode=mode, error=result["error"]))
        return results

    def invalid_original(self):
        status = "invalid_original" if self.case["origin"] == "reference" else "invalid_mutation"
        return [finding(self.case, "original_invalid_ruby", status, details=self.ruby["original_compile"])]

    def diagnostics(self):
        original = self.native[(self.case["id"], "off")]
        if original.get("error"):
            return []
        return compare_diagnostics(self.case, original, self.ruby, self.manual_guard())

    def manual_guard(self):
        actual = self.native[(self.case["id"], "all")]
        return manual_reference_guard(self.case, actual, self.ruby["all"])

    def reference_error(self):
        error = self.ruby["error"]
        invalid = self.case["origin"] != "reference" and "Invalid Ruby syntax" in error
        return [finding(self.case, "reference_error", "invalid_mutation" if invalid else "failure", error=error)]

    def correction(self, mode):
        actual = self.native[(self.case["id"], mode)]
        if actual.get("error"):
            return []
        results = correction_result(self.case, mode, actual, self.ruby[mode])
        results.extend(syntax_results(self.case, mode, self.ruby, actual))
        return results
