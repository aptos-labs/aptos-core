from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from harness.agent import FakeAgentSession
from harness.artifacts import JsonlWriter
from harness.judge import JudgeResult
from harness.state_machine import ConversationPolicy


ROOT = Path(__file__).resolve().parent.parent
ARMS = ("agent_only", "hybrid_guided", "hybrid_flexible")


def result(state: str, diagnostics: str = "diagnostic") -> JudgeResult:
    return JudgeResult(
        schema_version=2,
        state=state,
        diagnostics=diagnostics,
        verdict={},
        command={},
        tree_sha256="0" * 64,
    )


class ConversationPolicyTest(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = ConversationPolicy(ROOT / "prompts" / "followups.json")

    def test_every_branch_is_arm_blind(self) -> None:
        states = (
            "infrastructure_failure",
            "compile_failure",
            "forbidden_weakening",
            "incomplete_contract",
            "weak_contract",
            "prover_failure",
            "prover_timeout",
            "no_progress",
            "operational_success",
        )
        for state in states:
            decisions = {
                arm: self.policy.decide(result(state), "module::function", 0)
                for arm in ARMS
            }
            self.assertEqual(1, len(set(decisions.values())), (state, decisions))

    def test_infrastructure_retries_identical_snapshot(self) -> None:
        decision = self.policy.decide(result("infrastructure_failure"), "target", 0)
        self.assertEqual("retry_snapshot", decision.action)
        self.assertIsNone(decision.prompt)

    def test_second_forbidden_result_stops(self) -> None:
        first = self.policy.decide(result("forbidden_weakening"), "target", 0)
        second = self.policy.decide(result("forbidden_weakening"), "target", 1)
        self.assertEqual("continue", first.action)
        self.assertEqual("repeated_forbidden_weakening", second.terminal_status)

    def test_a_weak_contract_continues_rather_than_stopping(self) -> None:
        # A verifying contract that a refutation survives is an ordinary repair
        # loop: the agent has to strengthen it, not undo a policy breach.
        first = self.policy.decide(result("weak_contract"), "target", 0)
        second = self.policy.decide(result("weak_contract"), "target", 1)
        self.assertEqual("continue", first.action)
        self.assertEqual("continue", second.action)
        self.assertIsNone(second.terminal_status)
        self.assertIn("too weak", second.prompt)

    def test_a_repeatedly_incomplete_contract_is_not_a_repeated_breach(self) -> None:
        # An incomplete contract is an ordinary repair loop: the author has to
        # add a clause. Only a genuine weakening spends the repeated-breach
        # stop rule, which previously ended such a run on its second attempt
        # under a status describing a policy violation that never happened.
        first = self.policy.decide(result("incomplete_contract"), "target", 0)
        second = self.policy.decide(result("incomplete_contract"), "target", 1)
        self.assertEqual("continue", first.action)
        self.assertEqual("continue", second.action)
        self.assertIsNone(second.terminal_status)
        self.assertIn("incomplete", second.prompt)


class FakeAgentBranchTest(unittest.IsolatedAsyncioTestCase):
    async def test_same_fake_turns_produce_same_branch_decisions_in_every_arm(self) -> None:
        states = (
            "infrastructure_failure",
            "compile_failure",
            "forbidden_weakening",
            "prover_failure",
            "prover_timeout",
            "no_progress",
            "operational_success",
        )
        policy = ConversationPolicy(ROOT / "prompts" / "followups.json")
        decisions_by_arm = {}
        responses_by_arm = {}
        with tempfile.TemporaryDirectory() as temporary:
            temporary_root = Path(temporary)
            for arm in ARMS:
                workspace = temporary_root / arm
                workspace.mkdir()
                with JsonlWriter(temporary_root / f"{arm}.jsonl") as events:
                    async with FakeAgentSession(
                        Path(__file__).parent
                        / "fixtures"
                        / "fake-agent-all-branches.json",
                        workspace,
                        events,
                    ) as agent:
                        responses = []
                        decisions = []
                        for state in states:
                            turn = await agent.send("identical prompt")
                            responses.append(turn.result)
                            decisions.append(policy.decide(result(state), "target", 0))
                        responses_by_arm[arm] = responses
                        decisions_by_arm[arm] = decisions
        self.assertEqual(1, len({tuple(value) for value in responses_by_arm.values()}))
        self.assertEqual(1, len({tuple(value) for value in decisions_by_arm.values()}))


if __name__ == "__main__":
    unittest.main()
