import unittest

from analysis.codex_model_comparison import model_differences


def cell(task, replicate, arm, strict, cost):
    return {"task_id": task, "replicate": replicate, "arm": arm,
            "strict_success": strict, "cost_usd": cost, "wall_seconds": 1.0}


class ModelDifferencesTest(unittest.TestCase):
    def test_pairs_cells_by_task_and_replicate_within_an_arm(self) -> None:
        first = [cell("T1", "1", "agent_only", False, 1.0), cell("T1", "2", "agent_only", True, 1.0),
                 cell("T2", "1", "agent_only", True, 2.0), cell("T1", "1", "hybrid_guided", False, 9.0)]
        second = [cell("T1", "1", "agent_only", True, 3.0), cell("T1", "2", "agent_only", True, 3.0),
                  cell("T2", "1", "agent_only", True, 2.0), cell("T1", "1", "hybrid_guided", True, 0.0)]
        strict = model_differences(first, second, "agent_only", "strict_success")
        # T1 gains one of two replicates (+0.5), T2 none: the equal-weight mean is +0.25.
        self.assertAlmostEqual(strict["mean"], 0.25)
        self.assertEqual((strict["pairs"], strict["tasks"]), (3, 2))
        cost = model_differences(first, second, "agent_only", "cost_usd")
        self.assertAlmostEqual(cost["mean"], (2.0 + 0.0) / 2)

    def test_unmeasured_cells_are_left_out(self) -> None:
        first = [cell("T1", "1", "agent_only", None, 1.0), cell("T1", "2", "agent_only", True, 1.0)]
        second = [cell("T1", "1", "agent_only", True, 1.0), cell("T1", "2", "agent_only", False, 1.0)]
        strict = model_differences(first, second, "agent_only", "strict_success")
        self.assertEqual(strict["pairs"], 1)
        self.assertAlmostEqual(strict["mean"], -1.0)


if __name__ == "__main__":
    unittest.main()
