import json


def read_results(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def finding(case, kind, status, **details):
    return {"id": case["id"], "cop": case["cop"], "kind": kind, "status": status, **details}


def identity(case):
    return json.dumps({k: case.get(k) for k in ("cop", "source", "path", "config", "runtime")}, sort_keys=True)
