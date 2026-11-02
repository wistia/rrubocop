import argparse
from pathlib import Path


def arguments():
    parser = argparse.ArgumentParser(description="Audit added cops against the pinned Ruby bundle")
    selection_options(parser)
    execution_options(parser)
    args = parser.parse_args()
    validate(parser, args)
    return args


def selection_options(parser):
    parser.add_argument("--ruby-project", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=Path("/tmp/rrubocop-parity-audit"))
    parser.add_argument("--cop", action="append", help="Repeat to select cops; default: all 98 additions")
    parser.add_argument("--case-id", help="Replay one ID from an identical corpus/mutation selection")
    parser.add_argument("--limit", type=int, help="Limit examples; coverage report marks partial runs")
    parser.add_argument("--mutations-per-cop", type=int, default=2, help="Seeds per cop; 0 disables mutations")


def execution_options(parser):
    parser.add_argument("--runtime-only", action="store_true", help="Run only explicit sandbox examples")
    parser.add_argument("--skip-runtime", action="store_true")
    parser.add_argument("--timeout", type=int, default=600, help="Timeout per engine batch")
    parser.add_argument("--fail-on-upstream-hazards", action="store_true")


def validate(parser, args):
    if args.mutations_per_cop < 0 or args.limit is not None and args.limit < 1:
        parser.error("mutation count must be nonnegative and limit must be positive")
