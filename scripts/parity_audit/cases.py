import copy
import hashlib
import json

from .common import identity
from .mutations import mutate




def read_fixture(root, name):
    return json.loads((root / "tests/fixtures" / f"{name}.json").read_text())


def fixtures(root):
    base = read_fixture(root, "ported-reference")
    names = sorted({case["cop"] for case in base})
    base.extend(extra_fixtures(root))
    return base, names


def extra_fixtures(root):
    extra = read_fixture(root, "final-parity-regressions")
    extra.extend(read_fixture(root, "scope-shape-directives"))
    extra.extend(json.loads((root / "scripts/parity_audit/runtime_cases.json").read_text()))
    return extra


def references(base, selected):
    unique = {}
    for original in base:
        if selected and original["cop"] not in selected:
            continue
        case = defaults(original)
        unique[identity(case)] = {**unique.get(identity(case), {}), **case}
    return unique


def defaults(original):
    case = copy.deepcopy(original)
    case.setdefault("expected", [])
    case.setdefault("corrected", case["source"])
    case.setdefault("config", {})
    case["origin"] = "reference"
    return case




def select(cases, args):
    if args.runtime_only:
        cases = [c for c in cases if "runtime" in c]
    if args.case_id:
        cases = [c for c in cases if c["id"] == args.case_id]
    if args.limit is not None:
        cases = cases[:args.limit]
    if not cases:
        raise ValueError("Selection contains no examples")
    return cases


def assign_ids(unique):
    cases = list(unique.values())
    for case in cases:
        case["id"] = hashlib.sha256(identity(case).encode()).hexdigest()[:16]
    return cases


def collect(root, args):
    base, names = fixtures(root)
    validate_names(args.cop, names)
    unique = references(base, args.cop)
    mutate(unique, args.mutations_per_cop)
    return assign_ids(unique), names


def validate_names(selected, names):
    if selected and set(selected) - set(names):
        raise ValueError(f"Unknown cops: {sorted(set(selected) - set(names))}")
