import hashlib
import json
import subprocess


def git(root, *args):
    return subprocess.check_output(["git", *args], cwd=root, text=True).strip()


def file_hash(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inputs(args, output):
    return {"bundle_lock_sha256": file_hash(args.ruby_project / "Gemfile.lock"),
            "cases_sha256": file_hash(output / "cases.json"),
            "selection": {key: str(value) if hasattr(value, "resolve") else value
                          for key, value in vars(args).items()}}


def record(args, root, output):
    data = {"native_head": git(root, "rev-parse", "HEAD"),
            "native_dirty": bool(git(root, "status", "--porcelain"))}
    data.update(inputs(args, output))
    (output / "run-meta.json").write_text(json.dumps(data, indent=2) + "\n")
