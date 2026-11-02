import json
import subprocess

from .common import finding
from .compare import load


def execute(case, source, project, root):
    payload = {**case["runtime"], "source": source}
    command = ["bundle", "exec", "ruby", str(root / "scripts/parity_audit/runtime.rb")]
    try:
        result = subprocess.run(command, cwd=project, input=json.dumps(payload),
                                capture_output=True, text=True, timeout=5, check=True)
        return {**json.loads(result.stdout), "process_stderr": result.stderr}
    except (subprocess.SubprocessError, ValueError) as error:
        return {"runner_error": str(error)}


def check_runtime(cases, output, project, root):
    native, ruby = load(cases, output)
    audit = RuntimeAudit(native, ruby, project, root)
    for case in cases:
        if "runtime" in case:
            audit.case(case)
    (output / "runtime.json").write_text(json.dumps(audit.raw, indent=2) + "\n")
    return audit.results


class RuntimeAudit:
    def __init__(self, native, ruby, project, root):
        self.native, self.ruby = native, ruby
        self.project, self.root = project, root
        self.raw, self.results = [], []

    def execute(self, case, source):
        return execute(case, source, self.project, self.root)

    def case(self, case):
        original = self.execute(case, case["source"])
        for mode in ("safe", "all"):
            reference = self.ruby[case["id"]]
            if mode in reference:
                self.mode(case, mode, original, reference[mode]["source"])

    def mode(self, case, mode, original, reference_source):
        source = self.native[(case["id"], mode)].get("source", case["source"])
        actual = self.execute(case, source)
        expected = self.execute(case, reference_source)
        self.record(case, mode, original, actual, expected)

    def record(self, case, mode, original, actual, expected):
        self.raw.append({"id": case["id"], "mode": mode, "original": original, "native": actual, "ruby": expected})
        result = outcome(case, mode, original, actual, expected)
        if result:
            self.results.append(result)


def outcome(case, mode, original, actual, expected):
    if any("runner_error" in row for row in (original, actual, expected)):
        return finding(case, "runtime_runner", "failure", mode=mode)
    if actual != original:
        status = "upstream_hazard" if actual == expected else "failure"
        return finding(case, "runtime_change", status, mode=mode,
                       original=original, actual=actual, ruby=expected)
    if expected != original:
        return finding(case, "upstream_runtime_change", "upstream_hazard", mode=mode)
    return None
