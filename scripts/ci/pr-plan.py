#!/usr/bin/env python3
"""
PR Plan: classify a diff against risk packs and estimate LEM cost.

Usage:
  python3 scripts/ci/pr-plan.py [--base BASE_SHA] [--head HEAD_SHA]
      [--labels LABEL1,LABEL2] [--json-out PATH] [--github-summary PATH]

Exits 0 always (advisory). Writes JSON to --json-out and a markdown summary
to --github-summary (or $GITHUB_STEP_SUMMARY if set and path not given).
"""

from __future__ import annotations
import argparse
import fnmatch
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
BUDGET_PATH = REPO_ROOT / "policy" / "ci-budget.toml"
RISK_PACKS_PATH = REPO_ROOT / "policy" / "ci-risk-packs.toml"
LANE_WHITELIST_PATH = REPO_ROOT / "policy" / "ci-lane-whitelist.toml"


def load_toml(path: Path) -> dict[str, Any]:
    with open(path, "rb") as f:
        return tomllib.load(f)


def changed_files(base: str, head: str) -> list[str]:
    """Return list of files changed between base and head."""
    try:
        result = subprocess.run(
            ["git", "diff", "--name-only", base, head],
            capture_output=True, text=True, check=True, cwd=REPO_ROOT
        )
        files = [f.strip() for f in result.stdout.splitlines() if f.strip()]
        return files
    except subprocess.CalledProcessError as exc:
        print(f"warning: git diff failed ({exc}); assuming full diff", file=sys.stderr)
        return []


def path_matches(path: str, patterns: list[str]) -> bool:
    for pat in patterns:
        if fnmatch.fnmatch(path, pat):
            return True
        # Also match as prefix
        if pat.endswith("/**") and path.startswith(pat[:-3]):
            return True
    return False


def match_paths(changed: list[str], patterns: list[str]) -> bool:
    for fpath in changed:
        if path_matches(fpath, patterns):
            return True
    return False


def all_paths_match(changed: list[str], patterns: list[str]) -> bool:
    return bool(changed) and all(path_matches(fpath, patterns) for fpath in changed)


def classify_diff(
    changed: list[str],
    risk_packs: dict[str, Any],
    labels: list[str],
    lane_whitelist: list[dict[str, Any]],
) -> dict[str, Any]:
    """
    Returns:
      matched_packs: list of pack names matched by diff
      docs_only: bool
      triggered_lanes: set of lane ids
      triggered_deep: set of lane ids (deep lanes from matched packs)
      label_triggered: set of lane ids triggered by labels
    """
    matched_packs: list[str] = []
    triggered_lanes: set[str] = set()
    triggered_deep: set[str] = set()
    label_triggered: set[str] = set()

    packs = risk_packs.get("risk_pack", {})
    docs_patterns = packs.get("docs_only", {}).get("paths", [])

    for pack_name, pack in packs.items():
        paths = pack.get("paths", [])
        if match_paths(changed, paths):
            matched_packs.append(pack_name)
            for lane in pack.get("lanes", []):
                triggered_lanes.add(lane)
            for lane in pack.get("deep_lanes", []):
                triggered_deep.add(lane)

    # docs_only must be computed from every changed path. A mixed docs +
    # governance/script diff must not be allowed to skip the Rust gate just
    # because it also touched markdown.
    docs_only = all_paths_match(changed, docs_patterns)

    # If nothing matched, treat as ordinary Rust
    if not matched_packs or (not docs_only and not triggered_lanes and not triggered_deep):
        triggered_lanes.update(["fast_checks", "standard_tests", "ci_success"])

    # Labels override
    label_set = set(labels)
    all_packs = packs.values()
    for pack in all_packs:
        pack_labels = set(pack.get("labels", []))
        if pack_labels & label_set:
            for lane in pack.get("lanes", []):
                triggered_lanes.add(lane)
            for lane in pack.get("deep_lanes", []):
                triggered_lanes.add(lane)
                triggered_deep.add(lane)
                label_triggered.add(lane)

    for lane in lane_whitelist:
        lane_labels = set(lane.get("labels", []))
        if lane_labels & label_set:
            lane_id = lane.get("id")
            if lane_id:
                triggered_lanes.add(lane_id)
                label_triggered.add(lane_id)

    if label_triggered:
        docs_only = False

    return {
        "matched_packs": matched_packs,
        "docs_only": docs_only,
        "triggered_lanes": sorted(triggered_lanes),
        "triggered_deep": sorted(triggered_deep),
        "label_triggered": sorted(label_triggered),
    }


def estimate_lem(
    triggered_lanes: list[str],
    lane_whitelist: list[dict[str, Any]],
    runner_multipliers: dict[str, float],
) -> dict[str, Any]:
    lane_by_id = {lane["id"]: lane for lane in lane_whitelist}
    total_lem = 0.0
    breakdown: list[dict[str, Any]] = []

    for lane_id in triggered_lanes:
        lane = lane_by_id.get(lane_id)
        if lane is None:
            continue
        runner = lane.get("runner", "ubuntu_latest")
        base_lem = lane.get("base_lem", 10)
        multiplier = runner_multipliers.get(runner, 1.0)
        lem = base_lem * multiplier
        total_lem += lem
        breakdown.append({
            "lane_id": lane_id,
            "display_name": lane.get("display_name", lane_id),
            "runner": runner,
            "base_lem": base_lem,
            "multiplier": multiplier,
            "lem": lem,
        })

    return {"total_lem": round(total_lem, 1), "breakdown": breakdown}


def lem_tier(lem: float, budget: dict[str, Any]) -> str:
    preferred = budget.get("preferred_default_lem", 25)
    limit = budget.get("default_limit_lem", 35)
    elevated = budget.get("elevated_limit_lem", 75)
    hard = budget.get("hard_limit_lem", 125)
    if lem <= preferred:
        return "green"
    if lem <= limit:
        return "ok"
    if lem <= elevated:
        return "warning"
    if lem <= hard:
        return "high-warning"
    return "over-ceiling"


def write_github_summary(
    path: str,
    classification: dict[str, Any],
    estimate: dict[str, Any],
    budget: dict[str, Any],
    labels: list[str],
) -> None:
    tier = lem_tier(estimate["total_lem"], budget)
    tier_icon = {
        "green": "🟢",
        "ok": "🟡",
        "warning": "🟠",
        "high-warning": "🔴",
        "over-ceiling": "⛔",
    }.get(tier, "⚪")

    docs_flag = " *(docs only)*" if classification["docs_only"] else ""
    packs = ", ".join(classification["matched_packs"]) or "*(unclassified — treated as ordinary Rust)*"

    lines = [
        "## PR Plan",
        "",
        f"**Risk packs:** {packs}{docs_flag}",
        f"**Labels:** {', '.join(labels) if labels else '*(none)*'}",
        f"**Estimated LEM:** {tier_icon} {estimate['total_lem']} LEM",
        "",
        "### Lane Breakdown",
        "",
        "| Lane | Runner | Base LEM | Multiplier | Est LEM |",
        "| ---- | ------ | -------: | ---------: | ------: |",
    ]
    for item in estimate["breakdown"]:
        lines.append(
            f"| {item['display_name']} | {item['runner']} "
            f"| {item['base_lem']} | {item['multiplier']}× | {item['lem']} |"
        )
    lines += [
        "",
        f"**Total: {estimate['total_lem']} LEM**",
        "",
    ]
    if tier in ("warning", "high-warning"):
        limit = budget.get("elevated_limit_lem" if tier == "high-warning" else "default_limit_lem", 35)
        lines.append(
            f"> **Warning:** Estimated LEM ({estimate['total_lem']}) exceeds "
            f"the {limit} LEM {'elevated ' if tier == 'high-warning' else ''}threshold. "
            "Consider adding `ci-budget-ack` label if this is expected."
        )
    elif tier == "over-ceiling":
        hard = budget.get("hard_limit_lem", 125)
        lines.append(
            f"> **Error:** Estimated LEM ({estimate['total_lem']}) exceeds the hard ceiling of "
            f"{hard} LEM. Add `full-ci` or `ci-budget-override` label to proceed."
        )

    content = "\n".join(lines) + "\n"
    with open(path, "a", encoding="utf-8") as f:
        f.write(content)


def main() -> None:
    parser = argparse.ArgumentParser(description="PR Plan: classify diff and estimate LEM")
    parser.add_argument("--base", default="origin/main")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--labels", default="")
    parser.add_argument("--json-out", default="")
    parser.add_argument("--github-summary", default="")
    args = parser.parse_args()

    labels = [l.strip() for l in args.labels.split(",") if l.strip()]

    budget = load_toml(BUDGET_PATH)
    risk_packs = load_toml(RISK_PACKS_PATH)
    whitelist = load_toml(LANE_WHITELIST_PATH)
    lane_list = whitelist.get("lane", [])
    runner_multipliers = budget.get("runner_multipliers", {})

    changed = changed_files(args.base, args.head)
    classification = classify_diff(changed, risk_packs, labels, lane_list)
    estimate = estimate_lem(
        classification["triggered_lanes"],
        lane_list,
        runner_multipliers,
    )
    known_lanes = {lane["id"] for lane in lane_list}
    referenced_lanes = set(classification["triggered_lanes"]) | set(classification["triggered_deep"])
    unknown_lanes = sorted(referenced_lanes - known_lanes)
    for lane_id in unknown_lanes:
        print(f"warning: risk pack references unknown CI lane `{lane_id}`", file=sys.stderr)

    result = {
        "schema_version": 1,
        "base": args.base,
        "head": args.head,
        "labels": labels,
        "changed_files_count": len(changed),
        "classification": classification,
        "estimate": estimate,
        "tier": lem_tier(estimate["total_lem"], budget),
        "unknown_lanes": unknown_lanes,
    }

    if args.json_out:
        out_path = Path(args.json_out)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2)
        print(f"ci-plan: wrote {out_path}", file=sys.stderr)

    summary_path = args.github_summary or os.environ.get("GITHUB_STEP_SUMMARY", "")
    if summary_path:
        write_github_summary(
            summary_path,
            classification,
            estimate,
            budget.get("budget", budget),
            labels,
        )

    # Print compact summary to stdout
    tier = result["tier"]
    docs = " [docs-only]" if classification["docs_only"] else ""
    packs = ",".join(classification["matched_packs"]) or "unclassified"
    print(f"PR Plan: {estimate['total_lem']} LEM | packs={packs}{docs} | tier={tier}")

    # Set GitHub Actions output variables
    ga_output = os.environ.get("GITHUB_OUTPUT", "")
    if ga_output:
        with open(ga_output, "a") as f:
            f.write(f"docs_only={'true' if classification['docs_only'] else 'false'}\n")
            f.write(f"estimated_lem={estimate['total_lem']}\n")
            f.write(f"tier={tier}\n")
            f.write(f"matched_packs={','.join(classification['matched_packs'])}\n")


if __name__ == "__main__":
    main()
