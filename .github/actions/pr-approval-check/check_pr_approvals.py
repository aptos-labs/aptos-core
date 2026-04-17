#!/usr/bin/env python3
"""
check_pr_approvals.py — Enforce path-based PR approval rules.

Usage:
    python3 check_pr_approvals.py \
        --rules   .github/actions/pr-approval-check/approval-rules.yaml \
        --repo    aptos-labs/aptos-core \
        --pr      <PR number> \
        --token   <GitHub token>

Exit codes:
    0  All applicable rules are satisfied.
    1  One or more rules are violated (or an error occurred).
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import subprocess
import sys
import urllib.error
import urllib.request
from typing import Any

import yaml


# ---------------------------------------------------------------------------
# Git helpers
# ---------------------------------------------------------------------------

def get_changed_files(base_branch: str = "origin/main") -> list[str]:
    """Return the list of files changed relative to the merge-base with *base_branch*."""
    try:
        merge_base = subprocess.check_output(
            ["git", "merge-base", "HEAD", base_branch],
            text=True,
        ).strip()
    except subprocess.CalledProcessError as exc:
        exit_fatal(f"Could not determine merge-base with '{base_branch}': {exc}")

    try:
        out = subprocess.check_output(
            ["git", "diff", "--name-only", merge_base],
            text=True,
        ).strip()
    except subprocess.CalledProcessError as exc:
        exit_fatal(f"Could not list changed files: {exc}")

    return [line for line in out.splitlines() if line]


# ---------------------------------------------------------------------------
# GitHub API helpers
# ---------------------------------------------------------------------------

def github_get(url: str, token: str) -> Any:
    """Make an authenticated GET request to the GitHub API and return parsed JSON."""
    req = urllib.request.Request(
        url,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as exc:
        exit_fatal(f"GitHub API error {exc.code} for {url}: {exc.reason}")


def get_pr_author(repo: str, pr_number: int, token: str) -> str:
    """Return the GitHub login of the PR author."""
    data = github_get(
        f"https://api.github.com/repos/{repo}/pulls/{pr_number}",
        token,
    )
    return data["user"]["login"]


def get_org_team_members(org: str, team_slug: str, token: str) -> list[str]:
    """Return the list of GitHub logins for all members of an org team."""
    members: list[str] = []
    page = 1
    while True:
        url = (
            f"https://api.github.com/orgs/{org}/teams/{team_slug}/members"
            f"?per_page=100&page={page}"
        )
        data = github_get(url, token)
        if not data:
            break
        members.extend(m["login"] for m in data)
        if len(data) < 100:
            break
        page += 1
    return members


def get_pr_reviews(repo: str, pr_number: int, token: str) -> dict[str, str]:
    """
    Return a mapping of reviewer login → most-recent review state.

    States emitted by the API: APPROVED, CHANGES_REQUESTED, COMMENTED, DISMISSED.
    Later reviews overwrite earlier ones for the same user (last review wins).
    """
    all_reviews: list[dict] = []
    page = 1
    while True:
        url = (
            f"https://api.github.com/repos/{repo}/pulls/{pr_number}/reviews"
            f"?per_page=100&page={page}"
        )
        data = github_get(url, token)
        if not data:
            break
        all_reviews.extend(data)
        if len(data) < 100:
            break
        page += 1

    reviews: dict[str, str] = {}
    for review in sorted(all_reviews, key=lambda r: r["id"]):
        reviews[review["user"]["login"]] = review["state"]
    return reviews


# ---------------------------------------------------------------------------
# Rule-checking logic
# ---------------------------------------------------------------------------

def expand_approvers(raw: list[str], groups: dict[str, list[str]]) -> set[str]:
    """
    Expand a list that may contain group references (@group-name) into a flat
    set of GitHub usernames.
    """
    result: set[str] = set()
    for entry in raw:
        if entry.startswith("@"):
            group_name = entry[1:]
            if group_name not in groups:
                exit_fatal(f"Approval rule references unknown group '@{group_name}'.")
            else:
                result.update(groups[group_name])
        else:
            result.add(entry)
    return result


def file_matches_any(filepath: str, patterns: list[str]) -> bool:
    """Return True if *filepath* matches any of the glob *patterns*."""
    for pattern in patterns:
        if fnmatch.fnmatch(filepath, pattern):
            return True
    return False


def check_rules(
    changed_files: list[str],
    rules: list[dict],
    groups: dict[str, list[str]],
    approvals: dict[str, str],
    pr_author: str,
) -> tuple[list[dict], list[dict]]:
    """
    Evaluate every rule against *changed_files* and *approvals*.

    Each rule's *required_approvers* is a list of clauses evaluated with AND
    semantics: every clause must independently be satisfied.  Within a clause,
    the *approvers* list is an OR pool — any member's approval counts, up to
    that clause's *min_approvals* (default: 1).

    Returns:
        (violations, applied_rules)
        violations    — rules that apply but are not satisfied.
        applied_rules — all rules that apply (satisfied or not).
    """
    # Build the set of users who gave an APPROVED review (excluding the PR
    # author, who cannot approve their own PR on GitHub).
    approved_by: set[str] = {
        login
        for login, state in approvals.items()
        if state == "APPROVED" and login != pr_author
    }

    violations: list[dict] = []
    applied: list[dict] = []

    for rule in rules:
        name: str = rule["name"]
        paths: list[str] = rule.get("paths", [])
        clauses: list[dict] = rule["required_approvers"]

        # Collect all changed files that are covered by this rule.
        matching_files = [f for f in changed_files if file_matches_any(f, paths)]
        if not matching_files:
            continue  # This rule does not apply to the current PR.

        # AND mode: every clause must be independently satisfied.
        clause_results = []
        for clause in clauses:
            pool = expand_approvers(clause["approvers"], groups)
            needed: int = clause["min_approvals"]
            qualifying = approved_by & pool
            clause_results.append({
                "pool": sorted(pool),
                "needed": needed,
                "qualifying": sorted(qualifying),
                "satisfied": len(qualifying) >= needed,
            })
        unsatisfied_clauses = [c for c in clause_results if not c["satisfied"]]
        applied_entry = {
            "rule": name,
            "description": rule["description"],
            "matching_files": matching_files,
            "clause_results": clause_results,
            "unsatisfied_clauses": unsatisfied_clauses,
        }

        applied.append(applied_entry)
        if unsatisfied_clauses:
            violations.append(applied_entry)

    return violations, applied


# ---------------------------------------------------------------------------
# Output helpers
# ---------------------------------------------------------------------------

def exit_fatal(msg: str) -> None:
    print(f"[approval-check] ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def section(title: str) -> None:
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print("=" * 60)


def validate_rules(rules: list[dict]) -> None:
    """Validate that every rule has all required fields. Calls exit_fatal on the first problem found."""
    missing_description = [r.get("name", f"rule[{i}]") for i, r in enumerate(rules) if not r.get("description")]
    if missing_description:
        exit_fatal(f"The following rule(s) are missing a required 'description' field: {missing_description}")
    missing_approvers = [r.get("name", f"rule[{i}]") for i, r in enumerate(rules) if not r.get("required_approvers")]
    if missing_approvers:
        exit_fatal(f"The following rule(s) are missing a required 'required_approvers' field: {missing_approvers}")
    missing_min = [
        f"{r.get('name', f'rule[{i}]')}.required_approvers[{j}]"
        for i, r in enumerate(rules)
        for j, clause in enumerate(r.get("required_approvers", []))
        if "min_approvals" not in clause or clause["min_approvals"] < 1
    ]
    if missing_min:
        exit_fatal(f"The following clause(s) are missing or have an invalid 'min_approvals' field (must be >= 1): {missing_min}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify that a PR satisfies path-based approval rules."
    )
    parser.add_argument(
        "--rules",
        default=".github/actions/pr-approval-check/approval-rules.yaml",
        help="Path to the YAML approval-rules file (default: .github/actions/pr-approval-check/approval-rules.yaml)",
    )
    parser.add_argument(
        "--repo",
        required=True,
        help="GitHub repository in owner/repo format (e.g. aptos-labs/aptos-core)",
    )
    parser.add_argument(
        "--pr",
        required=True,
        type=int,
        help="Pull-request number",
    )
    parser.add_argument(
        "--token",
        required=True,
        help="GitHub API token with pull-request read access (secrets.GITHUB_TOKEN is sufficient)",
    )
    parser.add_argument(
        "--org-token",
        required=True,
        help="GitHub API token with read:org scope, used to fetch org team membership (must be a PAT or app token)",
    )
    parser.add_argument(
        "--base-branch",
        default="origin/main",
        help="Branch to diff against when computing changed files (default: origin/main)",
    )
    parser.add_argument(
        "--org",
        default="",
        help=(
            "GitHub organization name used to resolve @team-name references that are not "
            "defined in the rules file (default: extracted from --repo, e.g. 'aptos-labs' "
            "from 'aptos-labs/aptos-core').  The token must have 'read:org' scope."
        ),
    )
    args = parser.parse_args()

    # ── Load rules ───────────────────────────────────────────────────────────
    try:
        with open(args.rules) as fh:
            config = yaml.safe_load(fh)
    except FileNotFoundError:
        exit_fatal(f"Rules file not found: {args.rules}")
    except yaml.YAMLError as exc:
        exit_fatal(f"Failed to parse rules file: {exc}")

    rules: list[dict] = config.get("rules", [])

    if not rules:
        print("[approval-check] No rules defined — nothing to check.")
        return

    # ── Resolve team references from GitHub org ──────────────────────────────
    # Collect every @team-name referenced in the rules and fetch its members
    # from the GitHub org teams API.
    org = args.org if args.org else args.repo.split("/")[0]
    referenced_groups: set[str] = set()
    for rule in rules:
        for clause in rule.get("required_approvers", []):
            for approver in clause.get("approvers", []):
                if approver.startswith("@"):
                    referenced_groups.add(approver[1:])

    groups: dict[str, list[str]] = {}
    if referenced_groups:
        print(f"[approval-check] Fetching {len(referenced_groups)} team(s) from GitHub org '{org}'…")
        for team_slug in sorted(referenced_groups):
            print(f"  → {org}/{team_slug}")
            members = get_org_team_members(org, team_slug, args.org_token)
            if not members:
                exit_fatal(
                    f"GitHub team '{org}/{team_slug}' was not found or has no members. "
                    f"Ensure the token has 'read:org' scope and the team slug is correct."
                )
            groups[team_slug] = members
            print(f"     {len(members)} member(s): {members}")

    # Validate that every rule has all required fields.
    validate_rules(rules)

    # ── Gather inputs ────────────────────────────────────────────────────────
    section("Gathering PR information")

    changed_files = get_changed_files(args.base_branch)
    print(f"Changed files ({len(changed_files)} total):")
    for f in changed_files:
        print(f"  {f}")

    pr_author = get_pr_author(args.repo, args.pr, args.token)
    print(f"\nPR author: {pr_author}")

    reviews = get_pr_reviews(args.repo, args.pr, args.token)
    approved_reviewers = [u for u, s in reviews.items() if s == "APPROVED" and u != pr_author]
    print(f"Approvals ({len(approved_reviewers)}): {approved_reviewers or '(none)'}")

    # ── Evaluate rules ───────────────────────────────────────────────────────
    section("Evaluating approval rules")

    violations, applied = check_rules(changed_files, rules, groups, reviews, pr_author)

    if not applied:
        print("No approval rules apply to the files changed in this PR.")
        return

    for entry in applied:
        status = "PASS" if entry not in violations else "FAIL"
        print(f"\n[{status}] {entry['rule']}")
        # Print description as a single line (strip whitespace/newlines from YAML block scalars).
        print(f"       {' '.join(entry['description'].split())}")
        # Show the first few matching files for context.
        shown = entry["matching_files"][:5]
        ellipsis = f" … (+{len(entry['matching_files']) - 5} more)" if len(entry["matching_files"]) > 5 else ""
        print(f"       Files: {shown}{ellipsis}")
        for i, clause in enumerate(entry["clause_results"], 1):
            clause_status = "ok" if clause["satisfied"] else "MISSING"
            label = f"Clause {i}" if len(entry["clause_results"]) > 1 else "Requires"
            print(
                f"       {label}: {clause['needed']} approval(s) from {clause['pool']}"
                f"  →  {clause_status} (have: {clause['qualifying'] or 'none'})"
            )

    # ── Final result ─────────────────────────────────────────────────────────
    section("Result")

    if violations:
        print(f"FAILED — {len(violations)} rule(s) not satisfied:\n")
        for v in violations:
            print(f"  • {v['rule']}")
            for clause in v["unsatisfied_clauses"]:
                print(
                    f"    Need {clause['needed']} approval(s) from {clause['pool']}"
                    f", have {clause['qualifying'] or 'none'}"
                )
        print()
        sys.exit(1)
    else:
        print(f"PASSED — all {len(applied)} applicable rule(s) satisfied.")


if __name__ == "__main__":
    main()
