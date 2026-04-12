#!/usr/bin/env python3
"""Unit tests for check_pr_approvals.py."""

import unittest
from unittest.mock import patch

from check_pr_approvals import (
    check_rules,
    expand_approvers,
    file_matches_any,
    get_org_team_members,
    get_pr_reviews,
    validate_rules,
)

GROUPS = {
    "consensus-team": ["alice", "bob"],
    "crypto-team": ["carol", "dave"],
}


# ---------------------------------------------------------------------------
# expand_approvers
# ---------------------------------------------------------------------------

class TestExpandApprovers(unittest.TestCase):

    def test_plain_username(self):
        self.assertEqual(expand_approvers(["alice"], {}), {"alice"})

    def test_group_reference(self):
        self.assertEqual(expand_approvers(["@consensus-team"], GROUPS), {"alice", "bob"})

    def test_mixed_users_and_groups(self):
        result = expand_approvers(["eve", "@consensus-team"], GROUPS)
        self.assertEqual(result, {"alice", "bob", "eve"})

    def test_multiple_groups_union(self):
        result = expand_approvers(["@consensus-team", "@crypto-team"], GROUPS)
        self.assertEqual(result, {"alice", "bob", "carol", "dave"})

    def test_same_user_in_two_groups_deduplicates(self):
        groups = {"a": ["alice", "bob"], "b": ["alice", "carol"]}
        result = expand_approvers(["@a", "@b"], groups)
        self.assertEqual(result, {"alice", "bob", "carol"})

    def test_empty_list(self):
        self.assertEqual(expand_approvers([], GROUPS), set())

    def test_unknown_group_dies(self):
        with self.assertRaises(SystemExit):
            expand_approvers(["@unknown-team"], GROUPS)


# ---------------------------------------------------------------------------
# file_matches_any
# ---------------------------------------------------------------------------

class TestFileMatchesAny(unittest.TestCase):

    def test_exact_match(self):
        self.assertTrue(file_matches_any("consensus/foo.rs", ["consensus/foo.rs"]))

    def test_double_star_matches_nested_path(self):
        self.assertTrue(file_matches_any("consensus/safety-rules/src/lib.rs", ["consensus/**"]))

    def test_double_star_matches_direct_child(self):
        self.assertTrue(file_matches_any("consensus/lib.rs", ["consensus/**"]))

    def test_similar_prefix_does_not_match(self):
        # "consensus_other/foo.rs" must not match "consensus/**"
        self.assertFalse(file_matches_any("consensus_other/foo.rs", ["consensus/**"]))

    def test_no_match_returns_false(self):
        self.assertFalse(file_matches_any("mempool/foo.rs", ["consensus/**"]))

    def test_empty_patterns_returns_false(self):
        self.assertFalse(file_matches_any("consensus/foo.rs", []))

    def test_first_of_multiple_patterns_matches(self):
        self.assertTrue(file_matches_any("consensus/foo.rs", ["consensus/**", "crypto/**"]))

    def test_second_of_multiple_patterns_matches(self):
        self.assertTrue(file_matches_any("crypto/foo.rs", ["consensus/**", "crypto/**"]))

    def test_star_matches_filename(self):
        self.assertTrue(file_matches_any("consensus/foo.rs", ["consensus/*.rs"]))

    def test_star_crosses_slash(self):
        # Python's fnmatch treats * as matching any character including /,
        # so "consensus/*.rs" matches files at any depth under consensus/.
        self.assertTrue(file_matches_any("consensus/src/foo.rs", ["consensus/*.rs"]))


# ---------------------------------------------------------------------------
# check_rules helpers
# ---------------------------------------------------------------------------

def make_rule(name, paths, clauses, description="A rule."):
    return {
        "name": name,
        "description": description,
        "paths": paths,
        "required_approvers": clauses,
    }


def make_clause(approvers, min_approvals):
    return {"approvers": approvers, "min_approvals": min_approvals}


# ---------------------------------------------------------------------------
# check_rules
# ---------------------------------------------------------------------------

class TestCheckRules(unittest.TestCase):

    # ── Rule applicability ────────────────────────────────────────────────────

    def test_no_changed_files_no_rules_applied(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        violations, applied = check_rules([], [rule], {}, {}, "author")
        self.assertEqual(applied, [])
        self.assertEqual(violations, [])

    def test_changed_file_not_matching_path_skips_rule(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        violations, applied = check_rules(["mempool/foo.rs"], [rule], {}, {}, "author")
        self.assertEqual(applied, [])

    def test_changed_file_matching_path_applies_rule(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        approvals = {"alice": "APPROVED"}
        _, applied = check_rules(["consensus/foo.rs"], [rule], {}, approvals, "author")
        self.assertEqual(len(applied), 1)
        self.assertEqual(applied[0]["rule"], "R")

    # ── Single clause pass / fail ─────────────────────────────────────────────

    def test_single_clause_satisfied(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"alice": "APPROVED"}, "author"
        )
        self.assertEqual(violations, [])

    def test_single_clause_no_approvals_is_violation(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        violations, _ = check_rules(["consensus/foo.rs"], [rule], {}, {}, "author")
        self.assertEqual(len(violations), 1)

    def test_min_approvals_2_with_1_approval_is_violation(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice", "bob"], 2)])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"alice": "APPROVED"}, "author"
        )
        self.assertEqual(len(violations), 1)

    def test_min_approvals_2_with_2_approvals_passes(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice", "bob"], 2)])
        approvals = {"alice": "APPROVED", "bob": "APPROVED"}
        violations, _ = check_rules(["consensus/foo.rs"], [rule], {}, approvals, "author")
        self.assertEqual(violations, [])

    # ── AND semantics (multiple clauses) ─────────────────────────────────────

    def test_and_both_clauses_satisfied(self):
        rule = make_rule("R", ["consensus/**"], [
            make_clause(["alice"], 1),
            make_clause(["carol"], 1),
        ])
        approvals = {"alice": "APPROVED", "carol": "APPROVED"}
        violations, _ = check_rules(["consensus/foo.rs"], [rule], {}, approvals, "author")
        self.assertEqual(violations, [])

    def test_and_first_clause_fails(self):
        rule = make_rule("R", ["consensus/**"], [
            make_clause(["alice"], 1),
            make_clause(["carol"], 1),
        ])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"carol": "APPROVED"}, "author"
        )
        self.assertEqual(len(violations), 1)
        unsatisfied = violations[0]["unsatisfied_clauses"]
        self.assertEqual(len(unsatisfied), 1)
        self.assertIn("alice", unsatisfied[0]["pool"])

    def test_and_second_clause_fails(self):
        rule = make_rule("R", ["consensus/**"], [
            make_clause(["alice"], 1),
            make_clause(["carol"], 1),
        ])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"alice": "APPROVED"}, "author"
        )
        self.assertEqual(len(violations), 1)
        unsatisfied = violations[0]["unsatisfied_clauses"]
        self.assertEqual(len(unsatisfied), 1)
        self.assertIn("carol", unsatisfied[0]["pool"])

    def test_and_both_clauses_fail_both_in_unsatisfied(self):
        rule = make_rule("R", ["consensus/**"], [
            make_clause(["alice"], 1),
            make_clause(["carol"], 1),
        ])
        violations, _ = check_rules(["consensus/foo.rs"], [rule], {}, {}, "author")
        self.assertEqual(len(violations), 1)
        self.assertEqual(len(violations[0]["unsatisfied_clauses"]), 2)

    # ── PR author exclusion ───────────────────────────────────────────────────

    def test_pr_author_approval_excluded(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["author"], 1)])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"author": "APPROVED"}, "author"
        )
        self.assertEqual(len(violations), 1)

    def test_pr_author_excluded_but_other_reviewer_counts(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["author", "alice"], 1)])
        approvals = {"author": "APPROVED", "alice": "APPROVED"}
        violations, _ = check_rules(["consensus/foo.rs"], [rule], {}, approvals, "author")
        self.assertEqual(violations, [])

    # ── Non-APPROVED review states ────────────────────────────────────────────

    def test_changes_requested_does_not_count(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"alice": "CHANGES_REQUESTED"}, "author"
        )
        self.assertEqual(len(violations), 1)

    def test_commented_does_not_count(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"alice": "COMMENTED"}, "author"
        )
        self.assertEqual(len(violations), 1)

    def test_dismissed_does_not_count(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"alice": "DISMISSED"}, "author"
        )
        self.assertEqual(len(violations), 1)

    # ── Last-review-wins ──────────────────────────────────────────────────────

    def test_last_review_wins_approved_then_changes_requested(self):
        # The approvals dict already reflects last-review-wins (built in get_pr_reviews),
        # so simulate a user whose final state is CHANGES_REQUESTED.
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"alice": "CHANGES_REQUESTED"}, "author"
        )
        self.assertEqual(len(violations), 1)

    def test_last_review_wins_changes_requested_then_approved(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], {}, {"alice": "APPROVED"}, "author"
        )
        self.assertEqual(violations, [])

    # ── Group expansion inside check_rules ───────────────────────────────────

    def test_group_reference_in_clause_expands(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["@consensus-team"], 1)])
        approvals = {"bob": "APPROVED"}
        violations, _ = check_rules(
            ["consensus/foo.rs"], [rule], GROUPS, approvals, "author"
        )
        self.assertEqual(violations, [])

    def test_unknown_group_in_clause_dies(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["@nonexistent"], 1)])
        with self.assertRaises(SystemExit):
            check_rules(["consensus/foo.rs"], [rule], {}, {}, "author")

    # ── Multiple rules ────────────────────────────────────────────────────────

    def test_multiple_rules_both_pass(self):
        rules = [
            make_rule("R1", ["consensus/**"], [make_clause(["alice"], 1)]),
            make_rule("R2", ["crypto/**"], [make_clause(["carol"], 1)]),
        ]
        approvals = {"alice": "APPROVED", "carol": "APPROVED"}
        violations, applied = check_rules(
            ["consensus/foo.rs", "crypto/bar.rs"], rules, {}, approvals, "author"
        )
        self.assertEqual(len(applied), 2)
        self.assertEqual(violations, [])

    def test_multiple_rules_one_fails(self):
        rules = [
            make_rule("R1", ["consensus/**"], [make_clause(["alice"], 1)]),
            make_rule("R2", ["crypto/**"], [make_clause(["carol"], 1)]),
        ]
        violations, applied = check_rules(
            ["consensus/foo.rs", "crypto/bar.rs"], rules, {}, {"alice": "APPROVED"}, "author"
        )
        self.assertEqual(len(applied), 2)
        self.assertEqual(len(violations), 1)
        self.assertEqual(violations[0]["rule"], "R2")

    def test_multiple_rules_only_one_applies(self):
        rules = [
            make_rule("R1", ["consensus/**"], [make_clause(["alice"], 1)]),
            make_rule("R2", ["crypto/**"], [make_clause(["carol"], 1)]),
        ]
        violations, applied = check_rules(
            ["consensus/foo.rs"], rules, {}, {"alice": "APPROVED"}, "author"
        )
        self.assertEqual(len(applied), 1)
        self.assertEqual(applied[0]["rule"], "R1")
        self.assertEqual(violations, [])

    # ── clause_results structure ──────────────────────────────────────────────

    def test_clause_results_structure(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice", "bob"], 2)])
        approvals = {"alice": "APPROVED"}
        _, applied = check_rules(["consensus/foo.rs"], [rule], {}, approvals, "author")
        cr = applied[0]["clause_results"][0]
        self.assertEqual(sorted(cr["pool"]), ["alice", "bob"])
        self.assertEqual(cr["needed"], 2)
        self.assertEqual(cr["qualifying"], ["alice"])
        self.assertFalse(cr["satisfied"])

    def test_matching_files_recorded(self):
        rule = make_rule("R", ["consensus/**"], [make_clause(["alice"], 1)])
        files = ["consensus/a.rs", "consensus/b.rs", "mempool/c.rs"]
        approvals = {"alice": "APPROVED"}
        _, applied = check_rules(files, [rule], {}, approvals, "author")
        self.assertEqual(sorted(applied[0]["matching_files"]), ["consensus/a.rs", "consensus/b.rs"])


# ---------------------------------------------------------------------------
# validate_rules
# ---------------------------------------------------------------------------

class TestValidateRules(unittest.TestCase):

    def _valid_rule(self, name="R"):
        return {
            "name": name,
            "description": "A rule.",
            "paths": ["foo/**"],
            "required_approvers": [{"approvers": ["alice"], "min_approvals": 1}],
        }

    def test_valid_rule_passes(self):
        validate_rules([self._valid_rule()])  # should not raise

    def test_missing_description_is_fatal(self):
        rule = self._valid_rule()
        del rule["description"]
        with self.assertRaises(SystemExit):
            validate_rules([rule])

    def test_missing_required_approvers_is_fatal(self):
        rule = self._valid_rule()
        del rule["required_approvers"]
        with self.assertRaises(SystemExit):
            validate_rules([rule])

    def test_missing_min_approvals_is_fatal(self):
        rule = self._valid_rule()
        del rule["required_approvers"][0]["min_approvals"]
        with self.assertRaises(SystemExit):
            validate_rules([rule])

    def test_min_approvals_zero_is_fatal(self):
        rule = self._valid_rule()
        rule["required_approvers"][0]["min_approvals"] = 0
        with self.assertRaises(SystemExit):
            validate_rules([rule])

    def test_min_approvals_negative_is_fatal(self):
        rule = self._valid_rule()
        rule["required_approvers"][0]["min_approvals"] = -1
        with self.assertRaises(SystemExit):
            validate_rules([rule])


# ---------------------------------------------------------------------------
# get_org_team_members
# ---------------------------------------------------------------------------

class TestGetOrgTeamMembers(unittest.TestCase):

    def _member(self, login):
        return {"login": login}

    @patch("check_pr_approvals.github_get")
    def test_single_page(self, mock_get):
        mock_get.return_value = [self._member("alice"), self._member("bob")]
        result = get_org_team_members("my-org", "my-team", "token")
        self.assertEqual(result, ["alice", "bob"])
        self.assertEqual(mock_get.call_count, 1)
        called_url = mock_get.call_args[0][0]
        self.assertIn("my-org", called_url)
        self.assertIn("my-team", called_url)

    @patch("check_pr_approvals.github_get")
    def test_empty_team_returns_empty_list(self, mock_get):
        mock_get.return_value = []
        result = get_org_team_members("my-org", "empty-team", "token")
        self.assertEqual(result, [])

    @patch("check_pr_approvals.github_get")
    def test_full_page_triggers_second_fetch(self, mock_get):
        page1 = [self._member(f"user{i}") for i in range(100)]
        page2 = [self._member("extra")]
        mock_get.side_effect = [page1, page2]
        result = get_org_team_members("my-org", "big-team", "token")
        self.assertEqual(mock_get.call_count, 2)
        self.assertIn("extra", result)
        self.assertEqual(len(result), 101)

    @patch("check_pr_approvals.github_get")
    def test_partial_second_page_stops_pagination(self, mock_get):
        page1 = [self._member(f"user{i}") for i in range(100)]
        page2 = [self._member("last1"), self._member("last2")]
        mock_get.side_effect = [page1, page2]
        result = get_org_team_members("my-org", "some-team", "token")
        self.assertEqual(mock_get.call_count, 2)
        self.assertEqual(len(result), 102)


# ---------------------------------------------------------------------------
# get_pr_reviews
# ---------------------------------------------------------------------------

class TestGetPrReviews(unittest.TestCase):

    def _review(self, login, state):
        return {"user": {"login": login}, "state": state}

    @patch("check_pr_approvals.github_get")
    def test_single_page(self, mock_get):
        mock_get.return_value = [self._review("alice", "APPROVED")]
        result = get_pr_reviews("owner/repo", 1, "token")
        self.assertEqual(result, {"alice": "APPROVED"})
        self.assertEqual(mock_get.call_count, 1)

    @patch("check_pr_approvals.github_get")
    def test_empty_first_page_returns_empty(self, mock_get):
        mock_get.return_value = []
        result = get_pr_reviews("owner/repo", 1, "token")
        self.assertEqual(result, {})

    @patch("check_pr_approvals.github_get")
    def test_full_page_triggers_second_fetch(self, mock_get):
        page1 = [self._review(f"user{i}", "APPROVED") for i in range(100)]
        page2 = [self._review("extra", "APPROVED")]
        mock_get.side_effect = [page1, page2]
        result = get_pr_reviews("owner/repo", 1, "token")
        self.assertEqual(mock_get.call_count, 2)
        self.assertIn("extra", result)

    @patch("check_pr_approvals.github_get")
    def test_last_review_wins(self, mock_get):
        # Reviews are returned in chronological order; later entries overwrite earlier.
        mock_get.return_value = [
            self._review("alice", "APPROVED"),
            self._review("alice", "CHANGES_REQUESTED"),
        ]
        result = get_pr_reviews("owner/repo", 1, "token")
        self.assertEqual(result["alice"], "CHANGES_REQUESTED")

    @patch("check_pr_approvals.github_get")
    def test_multiple_reviewers_tracked_independently(self, mock_get):
        mock_get.return_value = [
            self._review("alice", "APPROVED"),
            self._review("bob", "CHANGES_REQUESTED"),
            self._review("carol", "COMMENTED"),
        ]
        result = get_pr_reviews("owner/repo", 1, "token")
        self.assertEqual(result["alice"], "APPROVED")
        self.assertEqual(result["bob"], "CHANGES_REQUESTED")
        self.assertEqual(result["carol"], "COMMENTED")


if __name__ == "__main__":
    unittest.main()
