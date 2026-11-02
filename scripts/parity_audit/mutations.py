import copy

from .common import identity
from .policy import REPORT_ONLY


def variants(source):
    yield "leading-comment", "# parity audit\n" + source
    yield "parentheses", "(\n" + source + "\n)\n"
    yield "argument-comment", source.replace("(", "(\n# parity audit\n", 1)



def seed_cases(unique, count):
    seeds = {}
    for case in list(unique.values()):
        used = seeds.get(case["cop"], 0)
        eligible = not case.get("reason") or case["cop"] in REPORT_ONLY
        if used < count and "runtime" not in case and eligible:
            seeds[case["cop"]] = used + 1
            yield case


def mutate(unique, count):
    for case in seed_cases(unique, count):
        for mutation, source in variants(case["source"]):
            if source != case["source"]:
                variant = mutated(case, mutation, source)
                unique.setdefault(identity(variant), variant)


def mutated(case, mutation, source):
    variant = {k: copy.deepcopy(case[k]) for k in ("cop", "path", "config")}
    variant.update(source=source, expected=[], corrected=source, origin=mutation)
    return variant

