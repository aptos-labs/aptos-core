import json
from pathlib import Path
import tempfile
import unittest

from analysis.agent_reports import collect, render


class AgentReportsTest(unittest.TestCase):
    def test_the_last_agent_result_is_the_report(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "manifest.json").write_text(json.dumps({"records": [
                {"task_id": "T-x-001", "function": "f", "module": "0x1::m"}]}))
            round_dir = root / "round"
            (round_dir / "schedule" / "runs").mkdir(parents=True)
            spec = {"run_id": "r1", "task_id": "T-x-001", "arm": "agent_only", "replicate": 1}
            (round_dir / "schedule" / "runs" / "r1.json").write_text(json.dumps(spec))
            (round_dir / "runs" / "r1").mkdir(parents=True)
            events = [{"event": "agent_result", "result": {"result": "first turn"}},
                      {"event": "judge_result"},
                      {"event": "agent_result", "result": {"result": "- **Result:** done"}}]
            (round_dir / "runs" / "r1" / "controller-events.jsonl").write_text(
                "".join(json.dumps(e) + "\n" for e in events))
            (round_dir / "mutation-summary.json").write_text(json.dumps({"runs": [
                {"run_id": "r1", "outcome": "scored", "strict_success": True}]}))

            rows = collect(round_dir, root / "manifest.json")

            self.assertEqual(rows[0]["report"], "- **Result:** done")
            self.assertEqual((rows[0]["target"], rows[0]["outcome"]), ("f", "scored"))
            text = render("round", rows)
            self.assertIn("### agent-only, replicate 1: strict success", text)
            self.assertIn("- **Result:** done", text)


if __name__ == "__main__":
    unittest.main()
