from collections import Counter

MODES = ("safe", "all")
ACTIVITY = ("positive_rewrites", "unchanged_negative", "unchanged_report_only", "unchanged_not_applied")


def counts(rows):
    return {**activity_counts(rows),
            "source_changed_attempts": sum(row["source_changed"] for row in rows),
            "positive_rewrite_cops": positive_cops(rows),
            "blocked_reasons": blocked_reasons(rows),
            "behavior_outcomes": dict(sorted(Counter(row["behavior"] for row in rows).items()))}


def activity_counts(rows):
    activity = Counter(row["activity"] for row in rows)
    return {name: activity[name] for name in (*ACTIVITY, "blocked")}


def positive_cops(rows):
    return sorted({row["cop"] for row in rows if row["activity"] == "positive_rewrites"})


def blocked_reasons(rows):
    return dict(sorted(Counter(row["blocked_reason"] for row in rows if row["activity"] == "blocked").items()))


def by_mode(rows):
    return {mode: counts([row for row in rows if row["mode"] == mode]) for mode in MODES}


def cop_counts(cop, cases, rows):
    selected = [row for row in rows if row["cop"] == cop]
    return {"scenarios": sum(case["cop"] == cop for case in cases), **by_mode(selected)}
