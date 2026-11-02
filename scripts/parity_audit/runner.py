import os
import subprocess

from .metadata import record


def run(command, cwd, output, timeout, env=None):
    with output.open("w") as log:
        subprocess.run(command, cwd=cwd, env=env, stdout=log, stderr=subprocess.STDOUT,
                       timeout=timeout, check=True)


def engines(args, root, output):
    record(args, root, output)
    clear_results(output)
    native_engine(args, root, output)
    ruby_engine(args, root, output)


def clear_results(output):
    for name in ("native.jsonl", "reference.jsonl", "summary.json", "report.md", "results.json", "runtime.json", "reference-meta.json"):
        (output / name).unlink(missing_ok=True)


def native_engine(args, root, output):
    env = {**os.environ, "RRUBOCOP_AUDIT_INPUT": str(output / "cases.json"),
           "RRUBOCOP_AUDIT_OUTPUT": str(output / "native.jsonl")}
    run(["cargo", "test", "--locked", "--lib", "custom_parity_audit_probe", "--", "--ignored"],
        root, output / "native.log", args.timeout, env)


def ruby_engine(args, root, output):
    project = args.ruby_project.resolve()
    command = ["bundle", "exec", "ruby", str(root / "scripts/parity_audit/reference.rb"),
               str(project / ".rubocop.yml"), str(output / "cases.json"), str(output / "reference.jsonl")]
    run(command, project, output / "reference.log", args.timeout)
