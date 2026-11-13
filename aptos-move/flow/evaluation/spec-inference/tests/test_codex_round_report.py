import json
from pathlib import Path
import tempfile
import unittest

from analysis.codex_round_report import (
    PRICING_FILE,
    block_differences,
    holm,
    model_price,
    price_request,
    randomization_p_value,
    report,
)


PRICING = json.loads(PRICING_FILE.read_text(encoding="utf-8"))
TERRA = model_price(PRICING, "gpt-5.6-terra")


def usage(input_tokens, cached=0, output=0, reasoning=0):
    return {
        "input_tokens": input_tokens,
        "cached_input_tokens": cached,
        "cache_write_input_tokens": 0,
        "output_tokens": output,
        "reasoning_output_tokens": reasoning,
    }


def request(sequence, turn, tokens, classification="turn_request"):
    return {
        "event": "codex_response_usage",
        "classification": classification,
        "attempt": 1,
        "controller_turn": turn,
        "sequence": sequence,
        "model": "gpt-5.6-terra",
        "usage": tokens,
    }


def write_round(root: Path, cells, max_wall_seconds=3600) -> None:
    """A minimal recorded round: `cells` maps run ids to (task, arm,
    replicate, strict, outcome, wall seconds, usage events)."""
    (root / "schedule" / "runs").mkdir(parents=True)
    (root / "config.json").write_text(
        json.dumps({"model": "gpt-5.6-terra", "max_wall_seconds": max_wall_seconds})
    )
    summary = {"runs": []}
    for run_id, (task, arm, replicate, strict, outcome, wall, events) in cells.items():
        spec = {
            "run_id": run_id, "task_id": task, "arm": arm, "replicate": replicate,
            "block": replicate, "order": 1,
        }
        (root / "schedule" / "runs" / f"{run_id}.json").write_text(json.dumps(spec))
        run_dir = root / "runs" / run_id
        run_dir.mkdir(parents=True)
        (run_dir / "run.json").write_text(json.dumps({"result": {
            "controller_wall_ms": wall * 1000, "attempts": 1, "operational_success": True,
            "terminal_status": "operational_success",
        }}))
        (run_dir / "codex-request-usage.jsonl").write_text(
            "".join(json.dumps(event) + "\n" for event in events)
        )
        summary["runs"].append({
            "run_id": run_id, "task_id": task, "arm": arm, "outcome": outcome,
            "strict_success": strict, "mutation_adequacy": 1.0 if strict else 0.0,
        })
    (root / "mutation-summary.json").write_text(json.dumps(summary))


class PriceRequestTest(unittest.TestCase):
    def test_reasoning_is_part_of_output_and_billed_once(self) -> None:
        priced = price_request(usage(1_000, output=100, reasoning=60), TERRA)
        self.assertAlmostEqual(priced["output_cost_usd"], 100 * 12.0 / 1e6)
        self.assertAlmostEqual(priced["cost_usd"], (1_000 * 2.0 + 100 * 12.0) / 1e6)

    def test_cached_input_is_a_subset_of_input(self) -> None:
        priced = price_request(usage(10_000, cached=8_000), TERRA)
        self.assertEqual(priced["fresh_input_tokens"], 2_000)
        self.assertAlmostEqual(priced["cost_usd"], (2_000 * 2.0 + 8_000 * 0.2) / 1e6)

    def test_long_context_tier_applies_to_a_request_above_the_threshold(self) -> None:
        priced = price_request(usage(300_000, cached=100_000, output=1_000), TERRA)
        self.assertTrue(priced["long_context"])
        self.assertAlmostEqual(
            priced["cost_usd"],
            (200_000 * 2.0 * 2 + 100_000 * 0.2 * 2 + 1_000 * 12.0 * 1.5) / 1e6,
        )

    def test_a_request_at_the_threshold_is_not_long_context(self) -> None:
        self.assertFalse(price_request(usage(272_000), TERRA)["long_context"])

    def test_inconsistent_usage_is_refused(self) -> None:
        with self.assertRaisesRegex(ValueError, "exceed input"):
            price_request(usage(100, cached=200), TERRA)


class RoundReportTest(unittest.TestCase):
    def test_a_turn_whose_requests_sum_above_the_threshold_is_not_long_context(self) -> None:
        # Three 100K-token requests in one controller turn: the turn's
        # reconciliation total is 300K, but no single request is long-context.
        events = [request(i, 1, usage(100_000, output=500)) for i in (1, 2, 3)]
        events.append({
            "event": "codex_usage_reconciliation", "controller_turn": 1, "matched": True,
            "expected": usage(300_000, output=1_500), "observed": usage(300_000, output=1_500),
        })
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "round"
            write_round(root, {"r1": ("T1", "agent_only", 1, True, "scored", 60, events)})
            data = report(root, root / "report")
            arm = data["arms"]["agent_only"]
            self.assertEqual(arm["long_context_requests"], 0)
            self.assertEqual(arm["canonical_requests"], 3)
            self.assertAlmostEqual(arm["cost_usd"], 3 * (100_000 * 2.0 + 500 * 12.0) / 1e6)
            self.assertEqual(data["audit"], [])

    def test_warmup_is_reported_apart_and_mismatches_are_audited(self) -> None:
        events = [
            request(1, 1, usage(6_000), classification="startup_warmup"),
            request(2, 1, usage(10_000, output=100)),
            {"event": "codex_usage_reconciliation", "controller_turn": 1, "matched": False},
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "round"
            write_round(root, {"r1": ("T1", "agent_only", 1, True, "scored", 60, events)})
            data = report(root, root / "report")
            arm = data["arms"]["agent_only"]
            self.assertAlmostEqual(arm["cost_usd"], (10_000 * 2.0 + 100 * 12.0) / 1e6)
            self.assertAlmostEqual(arm["warmup_cost_usd"], 6_000 * 2.0 / 1e6)
            self.assertEqual(
                [entry["issue"] for entry in data["audit"]], ["usage_reconciliation_mismatch"]
            )
            for name in ("cells.csv", "requests.csv", "analysis.json", "pricing.json", "REPORT.md"):
                self.assertTrue((root / "report" / name).is_file(), name)

    def test_unmeasured_outcomes_do_not_count_as_failures(self) -> None:
        events = [request(1, 1, usage(1_000))]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "round"
            write_round(root, {
                "r1": ("T1", "agent_only", 1, True, "scored", 60, events),
                "r2": ("T1", "agent_only", 2, False, "inconclusive", 60, events),
                "r3": ("T1", "agent_only", 3, False, "disqualified", 60, events),
            })
            arm = report(root, root / "report")["arms"]["agent_only"]
            self.assertEqual(
                (arm["strict_successes"], arm["measured"], arm["unmeasured"], arm["disqualified"]),
                (1, 2, 1, 1),
            )

    def test_failures_are_censored_at_the_wall_cap(self) -> None:
        events = [request(1, 1, usage(1_000))]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "round"
            write_round(root, {
                "r1": ("T1", "agent_only", 1, True, "scored", 100, events),
                "r2": ("T1", "agent_only", 2, False, "scored", 50, events),
            }, max_wall_seconds=1000)
            interval = report(root, root / "report")["arms"]["agent_only"]["task_clustered"][
                "restricted_time_to_success_seconds"
            ]
            self.assertAlmostEqual(interval["mean"], (100 + 1000) / 2)


class ContrastTest(unittest.TestCase):
    def cells(self, effect: float):
        rows = []
        for task in range(8):
            for replicate in (1, 2):
                for arm, value in (("agent_only", 0.0), ("hybrid_flexible", effect)):
                    rows.append({"task_id": f"T{task}", "replicate": replicate, "arm": arm,
                                 "strict_success": value})
        return rows

    def test_block_differences_pair_arms_within_blocks(self) -> None:
        differences = block_differences(
            self.cells(1.0), "strict_success", "hybrid_flexible", "agent_only"
        )
        self.assertEqual(len(differences), 8)
        self.assertTrue(all(values == [1.0, 1.0] for values in differences.values()))

    def test_randomization_test_separates_effect_from_none(self) -> None:
        effect = block_differences(self.cells(1.0), "strict_success", "hybrid_flexible", "agent_only")
        none = block_differences(self.cells(0.0), "strict_success", "hybrid_flexible", "agent_only")
        self.assertLess(randomization_p_value(effect), 0.001)
        self.assertEqual(randomization_p_value(none), 1.0)

    def test_holm_adjustment(self) -> None:
        adjusted = holm({"C1": 0.01, "C2": 0.04})
        self.assertAlmostEqual(adjusted["C1"], 0.02)
        self.assertAlmostEqual(adjusted["C2"], 0.04)
        self.assertEqual(holm({"C1": None, "C2": 0.03}), {"C1": None, "C2": 0.03})


if __name__ == "__main__":
    unittest.main()
