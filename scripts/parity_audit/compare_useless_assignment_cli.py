"""Compare CLI corrections without executing any fixture programs."""

import argparse
import concurrent.futures
import json
import os
import pathlib
import subprocess
import sys
import tempfile

VALID_EXITS = (0, 1)


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cop", default="Lint/UselessAssignment")
    parser.add_argument("--fixture", type=pathlib.Path, nargs="+", required=True)
    parser.add_argument("--rust", type=pathlib.Path, required=True)
    parser.add_argument("--ruby", type=pathlib.Path, required=True)
    parser.add_argument("--gemfile", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--jobs", type=int, default=4)
    return parser.parse_args()


def inspected_files(completed):
    try:
        report = json.loads(completed.stdout)
    except ValueError:
        return None
    return report.get("summary", {}).get("inspected_file_count", report.get("metadata", {}).get("files_inspected"))


def observe(executable, options, path, source, cwd, environment):
    path.write_text(source)
    command = [str(executable), *options, str(path)]
    first = subprocess.run(command, cwd=cwd, env=environment, capture_output=True, text=True, timeout=30)
    corrected = path.read_bytes()
    second = subprocess.run(command, cwd=cwd, env=environment, capture_output=True, text=True, timeout=30)
    stderr = first.stderr + second.stderr
    cop_error = (
        "Infinite loop detected in" in stderr
        or ("An error occurred while" in stderr and "cop was inspecting" in stderr)
    )
    return {
        "exit": first.returncode,
        "inspected_files": inspected_files(first),
        "final_inspected_files": inspected_files(second),
        "final_exit": second.returncode,
        "bytes": corrected.decode(),
        "idempotent": corrected == path.read_bytes(),
        "stderr": stderr,
        "cop_error": cop_error,
    }


def compare(index_case, settings, environment):
    index, case = index_case
    rows = []
    with tempfile.TemporaryDirectory() as directory:
        root = pathlib.Path(directory)
        config = root / "config.yml"
        fixture_path = pathlib.PurePosixPath(case.get("path") or "input.rb")
        if fixture_path.name == "(string)":
            fixture_path = pathlib.PurePosixPath("input.rb")
        if not fixture_path.name or fixture_path.is_absolute() or ".." in fixture_path.parts:
            raise ValueError(f"CLI fixtures require a relative path beneath the temporary root: {fixture_path}")
        path = root.joinpath(*fixture_path.parts)
        path.parent.mkdir(parents=True, exist_ok=True)
        options = {key: value for key, value in case["config"].items() if value is not None and key != "TargetRubyVersion"}
        document = {
            "AllCops": {
                "DisabledByDefault": True,
                "NewCops": "disable",
                "TargetRubyVersion": case["config"]["TargetRubyVersion"],
            },
            settings.cop: options | {"Enabled": True},
        }
        document.update(case.get("sibling_cops", {}))
        config.write_text(json.dumps(document))
        for mode in ("-a", "-A"):
            cli_options = ["--config", str(config), "--only", settings.cop, "--cache", "false", "--format", "json", mode]
            observations = {
                name: observe(executable, cli_options, path, case["original"], settings.ruby.parent.parent, environment)
                for name, executable in (("ruby", settings.ruby), ("rust", settings.rust))
            }
            rows.append({
                "case": index,
                "id": case["id"],
                "original": case["original"],
                "mode": mode,
                "equal_scope": all(
                    observations["ruby"][key] is not None
                    and observations["ruby"][key] == observations["rust"][key]
                    for key in ("inspected_files", "final_inspected_files")
                ),
                "equal_bytes": observations["ruby"]["bytes"] == observations["rust"]["bytes"],
                "equal_exit": observations["ruby"]["exit"] == observations["rust"]["exit"],
                "observations": observations,
            })
    return rows


def differs(row):
    return not row["equal_bytes"] or not row["equal_exit"] or not row["equal_scope"] or any(
        not observation["idempotent"] or observation["exit"] not in VALID_EXITS or observation["final_exit"] not in VALID_EXITS or observation["cop_error"]
        for observation in row["observations"].values()
    )


def main():
    settings = arguments()
    settings.ruby, settings.rust = settings.ruby.resolve(), settings.rust.resolve()
    environment = dict(os.environ)
    if settings.gemfile:
        environment["BUNDLE_GEMFILE"] = str(settings.gemfile.resolve())
    cases = [case for fixture in settings.fixture for case in json.loads(fixture.read_text())]
    rows = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=settings.jobs) as pool:
        pending = pool.map(lambda case: compare(case, settings, environment), enumerate(cases))
        for completed in pending:
            rows.extend(completed)
            if len(rows) % 40 == 0:
                print(f"{len(rows)} native mode comparisons complete", flush=True)
    settings.output.write_text(json.dumps(rows, indent=2) + "\n")
    print(json.dumps({
        "mode_comparisons": len(rows),
        "equal_bytes": sum(row["equal_bytes"] for row in rows),
        "equal_scope": sum(row["equal_scope"] for row in rows),
        "equal_exit": sum(row["equal_exit"] for row in rows),
        "ruby_idempotent": sum(row["observations"]["ruby"]["idempotent"] for row in rows),
        "rust_idempotent": sum(row["observations"]["rust"]["idempotent"] for row in rows),
        "differences": sum(differs(row) for row in rows),
    }), flush=True)
    for row in rows:
        if differs(row):
            print("DIFFERENCE", row["case"], row["mode"], repr(row["original"]), row["observations"], flush=True)
    return int(any(differs(row) for row in rows))


if __name__ == "__main__":
    sys.exit(main())
