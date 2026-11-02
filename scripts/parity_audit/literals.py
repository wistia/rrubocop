import json

from .cases import assign_ids
from .common import identity
from .mutations import seed_cases
from .runner import run


def expand(cases, args, root, output):
    if args.runtime_only or args.mutations_per_cop == 0:
        return cases
    seeds = selected_seeds(cases, args.mutations_per_cop)
    return merge(cases, generate(seeds, args, root, output))


def selected_seeds(cases, count):
    references = {identity(c): c for c in cases if c["origin"] == "reference"}
    return list(seed_cases(references, count))


def generate(seeds, args, root, output):
    prepare_seeds(seeds, output)
    run(command(root, output), args.ruby_project.resolve(), output / "mutations.log", args.timeout)
    return json.loads((output / "literal-variants.json").read_text())


def prepare_seeds(seeds, output):
    (output / "literal-variants.json").unlink(missing_ok=True)
    (output / "literal-seeds.json").write_text(json.dumps(seeds))


def command(root, output):
    return ["bundle", "exec", "ruby", str(root / "scripts/parity_audit/literals.rb"),
               str(output / "literal-seeds.json"), str(output / "literal-variants.json")]


def merge(cases, variants):
    unique = {identity(c): c for c in cases}
    for variant in variants:
        unique.setdefault(identity(variant), variant)
    return assign_ids(unique)
