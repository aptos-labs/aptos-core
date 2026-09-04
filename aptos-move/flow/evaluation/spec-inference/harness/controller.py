"""Multi-turn, condition-blind specification-inference experiment controller."""

from __future__ import annotations

import argparse
import asyncio
from contextlib import asynccontextmanager, suppress
import hashlib
import json
import os
import shutil
import subprocess
import time
import dataclasses
from dataclasses import asdict
from pathlib import Path
from typing import Any

from .agent import AgentSession, AgentTurn, ClaudeAgentSession, FakeAgentSession
from .boogie_proxy import BoogieProxy
from .artifacts import (
    JsonlWriter,
    canonical_json,
    copy_final_tree,
    copy_snapshot,
    load_object,
    sha256_file,
    tree_hash,
    workspace_diff,
    write_json,
)
from .config import ARM_TO_TACTIC, ExperimentConfig, ResolvedRunSpec, RunSpec
from .credentials import redact_tree
from .identifiers import resolve_within
from .judge import Judge, JudgeResult
from .state_machine import ConversationPolicy
from .mutants import (
    NO_MUTANTS,
    mutation_fingerprint,
    inconclusive_mutants,
    overlapping_mutations,
    refutation_confirms,
    reached_a_verdict,
    run_mutant_cases,
    score_mutants,
)
from .compatibility import changed_stages, tool_executables
from .materialize import materialize_task


def _within(root: Path, relative: str) -> Path:
    # `.` is legitimate here: the task package may be the whole tree. Nothing
    # is deleted through this path -- it only names where the package is.
    return resolve_within(root, relative, "package_relpath", allow_root=True)


class Controller:
    def __init__(
        self,
        config: ExperimentConfig,
        run: ResolvedRunSpec,
        artifact_dir: Path,
        prompts_dir: Path,
        agent_kind: str,
        fake_script: Path | None,
        hidden_mutants: Path | None,
        refutation_mutants: Path | None = None,
    ):
        self.config = config
        self.run = run
        self.artifact_dir = artifact_dir
        self.prompts_dir = prompts_dir
        self.agent_kind = agent_kind
        self.fake_script = fake_script
        self.hidden_mutants = hidden_mutants
        #: Mutants used as *feedback* during the run, as opposed to
        #: `hidden_mutants`, which score it afterwards. Never the same set: a
        #: contract repaired against a mutant it was shown cannot also be
        #: measured by it.
        self.refutation_mutants = refutation_mutants
        self.workspace_root = artifact_dir / "workspace"
        self.baseline_root = artifact_dir / "baseline"
        self.plugin_dir = artifact_dir / "plugin"
        # The manifest supplies this, and both trees are derived from it: a
        # value climbing out would point the pristine baseline at the agent's
        # own workspace, and the judge would compare the workspace to itself.
        self.package = _within(self.workspace_root, run.spec.package_relpath)
        self.baseline_package = _within(self.baseline_root, run.spec.package_relpath)
        self.flow_events = artifact_dir / "flow-events.jsonl"
        self.runtime_mcp = artifact_dir / "mcp.runtime.json"
        self.candidate_check = artifact_dir / "candidate-check.json"
        self.policy = ConversationPolicy(prompts_dir / "followups.json")
        self.judge = Judge(config, artifact_dir / "judge-work")
        self.started_ns = time.monotonic_ns()

    async def run_experiment(self) -> dict[str, Any]:
        self._prepare_artifacts()
        output_tokens_by_attempt: list[int] = []
        forbidden_repairs = 0
        final_judge: JudgeResult | None = None
        terminal_status = "budget_exhausted"
        attempts = 0
        initial_prompt = self._initial_prompt()
        with JsonlWriter(self.artifact_dir / "controller-events.jsonl") as controller_events, JsonlWriter(
            self.artifact_dir / "claude-events.jsonl"
        ) as claude_events:
            controller_events.emit(
                "run_start",
                run_id=self.run.spec.run_id,
                task_id=self.run.spec.task_id,
                arm=self.run.spec.arm,
                replicate=self.run.spec.replicate,
            )
            prompt = initial_prompt
            while attempts <= self.config.infrastructure_retries:
                if attempts:
                    self._reset_workspace()
                    prompt = initial_prompt
                    forbidden_repairs = 0
                    # Per-attempt state, like the prompt and the repair count.
                    # A retry that ends without producing a verdict -- a wall
                    # budget reached before any judging -- would otherwise be
                    # read through the previous attempt's `infrastructure_
                    # failure`, overwriting its true terminal status and, at
                    # two such cells, tripping the dispatch abort.
                    final_judge = None
                    controller_events.emit("infrastructure_retry", attempt=attempts + 1)
                attempts += 1
                output_tokens_by_attempt.append(0)
                flow_events_offset = _count_lines(self.flow_events)
                try:
                    async with self._agent_boogie_proxy(), self._make_agent(
                        claude_events, attempts
                    ) as agent:
                        for controller_turn in range(1, self.config.max_controller_turns + 1):
                            if self._wall_seconds() >= self.config.max_wall_seconds:
                                terminal_status = "wall_budget_exhausted"
                                break
                            previous_hash = tree_hash(self.package)
                            controller_events.emit(
                                "prompt",
                                controller_turn=controller_turn,
                                prompt=prompt,
                                previous_tree_sha256=previous_hash,
                            )
                            remaining_wall_seconds = (
                                self.config.max_wall_seconds - self._wall_seconds()
                            )
                            try:
                                turn, deadline_hit = await _send_with_wall_deadline(
                                    agent, prompt, remaining_wall_seconds
                                )
                            except TimeoutError:
                                terminal_status = "wall_budget_exhausted"
                                controller_events.emit(
                                    "wall_budget_exhausted",
                                    controller_turn=controller_turn,
                                )
                                break
                            self._validate_runtime_identity(turn.system_init)
                            output_tokens = int(turn.usage.get("output_tokens", 0))
                            output_tokens_by_attempt[-1] += output_tokens
                            controller_events.emit(
                                "agent_result",
                                controller_turn=controller_turn,
                                result=asdict(turn),
                                accumulated_output_tokens=output_tokens_by_attempt[-1],
                                interrupted_at_wall_deadline=deadline_hit,
                            )
                            if deadline_hit:
                                terminal_status = "wall_budget_exhausted"
                                controller_events.emit(
                                    "wall_budget_exhausted",
                                    controller_turn=controller_turn,
                                    detail="turn interrupted; usage telemetry preserved",
                                )
                                break
                            self._validate_flow_configuration(flow_events_offset)
                            restart_error = self._flow_restart_error(flow_events_offset)
                            if restart_error is not None:
                                controller_events.emit(
                                    "flow_supervisor_restart",
                                    controller_turn=controller_turn,
                                    detail=restart_error,
                                )
                            agent_error = (
                                _agent_infrastructure_error(turn)
                                or restart_error
                                or self._flow_infrastructure_error(flow_events_offset)
                            )
                            if agent_error is not None:
                                final_judge = _infrastructure_result(agent_error)
                            else:
                                final_judge = await self.judge.evaluate(
                                    self.baseline_package,
                                    self.package,
                                    self.run.spec.target,
                                    self.run.spec.allowed_edit_paths,
                                    self.run.spec.required_contract_categories,
                                    previous_hash,
                                )
                            # A verifying contract may still be too weak: the
                            # loop otherwise stops the moment the prover is
                            # satisfied, which is exactly when an
                            # under-specified contract looks finished. Refuting
                            # it against the mutant set turns "it verifies" into
                            # "it verifies and rejects what it should". Scored
                            # here in the controller's own space -- the set is
                            # never mounted where the session can read it.
                            # Naming the unconstrained obligation categories is
                            # acceptance-level feedback: it tells the session
                            # which of the task's required categories its
                            # contract fails to pin. A control arm is defined by
                            # receiving compiler and prover answers only, so
                            # refuting there would give the baseline the
                            # treatment and make the comparison meaningless.
                            if (
                                final_judge.state == "operational_success"
                                and self.refutation_mutants is not None
                                and self._feedback_level() != "baseline"
                            ):
                                final_judge = await self._refute(
                                    final_judge, controller_events, controller_turn
                                )
                            write_json(
                                self.artifact_dir
                                / f"judge-attempt-{attempts}-turn-{controller_turn}.json",
                                asdict(final_judge),
                            )
                            controller_events.emit(
                                "judge_result",
                                controller_turn=controller_turn,
                                state=final_judge.state,
                                diagnostics=final_judge.diagnostics,
                                tree_sha256=final_judge.tree_sha256,
                            )
                            decision = self.policy.decide(
                                final_judge,
                                self.run.spec.target,
                                forbidden_repairs,
                            )
                            controller_events.emit(
                                "transition",
                                controller_turn=controller_turn,
                                judge_state=final_judge.state,
                                decision=asdict(decision),
                            )
                            if final_judge.state == "forbidden_weakening":
                                forbidden_repairs += 1
                            if decision.action == "retry_snapshot":
                                break
                            # The cap is a declared budget: a turn that crossed it
                            # is exhausted even when its verdict would have ended
                            # the run as a success.
                            # Retries add: the cap is on the run, not the attempt.
                            if sum(output_tokens_by_attempt) >= self.config.max_output_tokens:
                                terminal_status = "output_token_budget_exhausted"
                                break
                            if decision.action == "stop":
                                terminal_status = decision.terminal_status or final_judge.state
                                break
                            prompt = decision.prompt or ""
                        else:
                            terminal_status = "controller_turn_budget_exhausted"
                    if final_judge is None or final_judge.state != "infrastructure_failure":
                        break
                except Exception as error:
                    final_judge = _infrastructure_result(f"agent/controller infrastructure error: {error}")
                    controller_events.emit("infrastructure_exception", error=repr(error), attempt=attempts)
                if attempts > self.config.infrastructure_retries:
                    terminal_status = "invalid_infrastructure_failure"
                    break

            eventual = None
            mutation_score = None
            if terminal_status == "operational_success":
                # The accepting turn judged this very tree. Re-judging it says
                # the same thing unless the task raised the solver budget, in
                # which case a target that only proves under the larger one has
                # to be given it.
                eventual_timeout = self._eventual_timeout()
                if (
                    final_judge is not None
                    and eventual_timeout <= self.config.operational_timeout_seconds
                    and final_judge.tree_sha256 == tree_hash(self.package)
                ):
                    eventual = final_judge
                else:
                    eventual = await self.judge.evaluate(
                        self.baseline_package,
                        self.package,
                        self.run.spec.target,
                        self.run.spec.allowed_edit_paths,
                        self.run.spec.required_contract_categories,
                        None,
                        timeout_seconds=eventual_timeout,
                    )
                write_json(self.artifact_dir / "judge-eventual.json", asdict(eventual))
                if self.hidden_mutants is not None:
                    if sha256_file(self.hidden_mutants) != self.run.spec.mutant_manifest_sha256:
                        raise ValueError("hidden mutant manifest hash disagrees with the run")
                    mutation_score = await score_mutants(
                        self.config,
                        self.package,
                        self.baseline_package,
                        self.run.spec.target,
                        self.hidden_mutants,
                        eventual_timeout,
                    )
                    write_json(self.artifact_dir / "mutation-score.json", mutation_score)
            result = {
                "schema_version": 1,
                "run_id": self.run.spec.run_id,
                "terminal_status": terminal_status,
                "attempts": attempts,
                "controller_wall_ms": (time.monotonic_ns() - self.started_ns) // 1_000_000,
                "total_output_tokens": sum(output_tokens_by_attempt),
                "output_tokens_by_attempt": output_tokens_by_attempt,
                "final_judge": asdict(final_judge) if final_judge else None,
                "eventual_judge": asdict(eventual) if eventual else None,
                "mutation_score": mutation_score,
                "operational_success": terminal_status == "operational_success",
                "scoring_mode": (
                    "reference_mutants" if self.hidden_mutants is not None else "core"
                ),
                "strict_success": bool(
                    terminal_status == "operational_success"
                    and mutation_score
                    and mutation_score["killed"] == mutation_score["essential_mutants"]
                ),
            }
            controller_events.emit("run_end", **result)
        self._finalize(result)
        return result

    async def _refute(
        self,
        judge: JudgeResult,
        controller_events: JsonlWriter,
        controller_turn: int,
    ) -> JudgeResult:
        """Downgrade an accepted candidate that fails to reject a mutant."""
        cases = load_object(self.refutation_mutants)["mutants"]
        # Refutation runs after the turn's judge, so the loop's wall check is
        # already behind it: without a bound here a cell can finish its budget
        # inside the mutant proofs and still record a success, which puts the
        # round's own runtime measurement outside the budget it declares.
        remaining = self.config.max_wall_seconds - self._wall_seconds()
        budget = min(self._eventual_timeout(), max(0, int(remaining / max(1, len(cases)))))
        if budget <= 0:
            # No budget is not a verdict. Saying so is what keeps an unmeasured
            # candidate from being confirmed by default.
            controller_events.emit(
                "refutation",
                controller_turn=controller_turn,
                killed=0,
                total=len(cases),
                survived=[],
                inconclusive=[case["mutant_id"] for case in cases],
            )
            return dataclasses.replace(
                judge,
                state="infrastructure_failure",
                diagnostics=(
                    "no wall budget left to refute the candidate, so it could "
                    "not be confirmed"
                ),
            )
        results = await run_mutant_cases(
            self.config,
            self.package,
            self.baseline_package,
            self.run.spec.target,
            cases,
            budget,
        )
        # Bounding each case is not bounding the run: the cases are sequential
        # and every command carries its own watchdog, so the total can overrun.
        overran = self._wall_seconds() >= self.config.max_wall_seconds
        survived = [r for r in results if r["outcome"] == "survived"]
        # A mutant that timed out, failed to compile or crashed the prover
        # produced no verdict. It is neither killed nor survived, so counting
        # it as neither means an all-inconclusive refutation leaves `survived`
        # empty -- and confirming the candidate on that basis would report "no
        # counterexample found" as "no counterexample exists".
        inconclusive = [r for r in results if not reached_a_verdict(r)]
        controller_events.emit(
            "refutation",
            controller_turn=controller_turn,
            killed=sum(1 for r in results if r["killed"]),
            total=len(results),
            survived=[r["mutant_id"] for r in survived],
            inconclusive=[r["mutant_id"] for r in inconclusive],
            overran_budget=overran,
        )
        # An overrun matters for whether the candidate can be *confirmed*, not
        # for a counterexample already found: a survivor is a verdict whatever
        # the clock did. Discarding it would drop the obligation categories the
        # session needs, and make a spent run-wide budget look retryable.
        if not survived and overran:
            return dataclasses.replace(
                judge,
                state="infrastructure_failure",
                diagnostics=(
                    "refutation ran past the run's wall budget, so the candidate "
                    "was not confirmed within the time the round declares"
                ),
            )
        if not survived and inconclusive:
            # Not a verdict on the specification, so not a specification state:
            # the round could not measure this cell.
            return dataclasses.replace(
                judge,
                state="infrastructure_failure",
                diagnostics=(
                    "refutation reached no verdict for "
                    f"{len(inconclusive)} of {len(results)} mutants "
                    f"({', '.join(sorted({r['outcome'] for r in inconclusive}))}), "
                    "so the candidate could not be confirmed"
                ),
            )
        if refutation_confirms(survived, inconclusive, overran):
            return judge
        # The category, not the mutant's name: `lost-element` states the defect
        # and would hand over the missing clause. The obligation it belongs to
        # says where to look without saying what to write.
        categories = sorted({
            next((c["obligation_category"] for c in cases if c["mutant_id"] == r["mutant_id"]), "unknown")
            for r in survived
        })
        return dataclasses.replace(
            judge,
            state="weak_contract",
            diagnostics=(
                f"{len(survived)} of {len(results)} refutations survive this "
                f"contract: an implementation that violates the intended "
                f"behaviour still satisfies it. Obligation categories left "
                f"unconstrained: {', '.join(categories)}."
            ),
        )

    def _eventual_timeout(self) -> int:
        """Solver budget for judging this run.

        A task may record its own, for a target whose proof does not fit the
        round's single budget. Raising the round's budget instead would slow
        every cell to accommodate one.
        """
        return self.run.spec.prove_timeout_seconds or self.config.eventual_timeout_seconds

    def _prepare_artifacts(self) -> None:
        if self.artifact_dir.exists():
            raise FileExistsError(f"refusing to overwrite run artifact `{self.artifact_dir}`")
        self.artifact_dir.mkdir(parents=True)
        (self.artifact_dir / "stdout.log").touch()
        (self.artifact_dir / "stderr.log").touch()
        # The sandbox grants the untrusted Claude/MCP process write access to
        # this file, not to the controller's artifact directory. Landlock path
        # rules require the file to exist before the agent process starts.
        self.flow_events.touch()
        manifest = self.run.plugin_dir / "move-flow-manifest.json"
        manifest_hash = sha256_file(manifest)
        if manifest_hash != self.run.spec.plugin_manifest_sha256:
            raise ValueError(
                f"plugin manifest hash mismatch: expected {self.run.spec.plugin_manifest_sha256}, got {manifest_hash}"
            )
        # The manifest is one file in the plugin. The skill this arm runs and
        # the `.mcp.json` that fixes its tool boundary are others, and the
        # session loads the whole directory.
        tree = tree_hash(self.run.plugin_dir)
        if tree != self.run.spec.plugin_tree_sha256:
            raise ValueError(
                f"plugin tree hash mismatch: expected {self.run.spec.plugin_tree_sha256}, got {tree}"
            )
        plugin_manifest = json.loads(manifest.read_text(encoding="utf-8"))
        if plugin_manifest.get("inference_tactic") != self.run.spec.arm:
            raise ValueError("run arm and generated plugin tactic disagree")
        if plugin_manifest.get("flow_source_commit") != self.config.source_commit:
            raise ValueError("generated plugin and experiment source commit disagree")
        if plugin_manifest.get("evaluation_mode") is not True:
            raise ValueError("experiment plugin was not generated in evaluation mode")
        if plugin_manifest.get("feedback_level") != self._feedback_level():
            raise ValueError(
                "generated plugin and scheduled feedback level disagree: "
                f"plugin {plugin_manifest.get('feedback_level')!r}, "
                f"run {self._feedback_level()!r}"
            )
        harness_hash = tree_hash(Path(__file__).resolve().parent)
        move_flow = _binary_identity("move-flow")
        self._validate_apparatus_identity(harness_hash, move_flow)
        source_hash = materialize_task(
            self.run.shared_package,
            self.run.task_patch,
            self.baseline_root,
            self.run.spec.initial_tree_sha256,
        )
        copy_snapshot(self.baseline_root, self.workspace_root)
        copy_snapshot(self.run.plugin_dir, self.plugin_dir)
        self._write_candidate_check()
        self._write_runtime_mcp()
        (self.artifact_dir / "initial-tree.sha256").write_text(source_hash + "\n", encoding="utf-8")
        run_record = {
            **asdict(self.run.spec),
            "config": asdict(self.config),
            "config_sha256": hashlib.sha256(canonical_json(asdict(self.config))).hexdigest(),
            "controller_harness_sha256": harness_hash,
            "controller_prompts_sha256": tree_hash(
                Path(__file__).resolve().parent.parent / "prompts"
            ),
            "plugin_manifest": plugin_manifest,
            "move_flow": move_flow,
            # What this run was *shown*. The scheduler proved it disjoint from
            # the scheduled scoring set, but a set corrected after the round
            # replaces that side of the comparison, and the replacement has
            # never been compared to anything. Recording the fingerprints lets
            # scoring re-check the relation instead of inheriting a guarantee
            # about a manifest it is no longer using. Opaque hashes, so the
            # refuted mutations still exist nowhere a session can read.
            "refutation_mutant_identities": self._refutation_identities(),
        }
        if (
            self.run.spec.experiment_config_sha256 is not None
            and run_record["config_sha256"]
            != self.run.spec.experiment_config_sha256
        ):
            raise ValueError(
                "run manifest and experiment configuration hash disagree"
            )
        write_json(self.artifact_dir / "run.json", run_record)

    def _refutation_identities(self) -> list[str]:
        """Fingerprints of the mutations this run may be shown, if any."""
        if self.refutation_mutants is None:
            return []
        return sorted(
            mutation_fingerprint(case, self.run.shared_package)
            for case in load_object(self.refutation_mutants)["mutants"]
        )

    def _initial_prompt(self) -> str:
        """Activate the selected plugin skill, then state the arm-blind task.

        This follows Flow's normal user-facing installation path: the generated
        plugin is passed as ``--plugin-dir`` and Claude resolves ``/move-inf``.
        The skill itself remains opaque to the controller prompt and comes
        solely from the selected Tera-rendered plugin tree.
        """
        task = (self.prompts_dir / "initial.txt").read_text(encoding="utf-8").format(
            target=self.run.spec.target,
            package=".",
        )
        return "/move-inf\n\n" + task

    def _validate_apparatus_identity(
        self, harness_hash: str, move_flow: dict[str, Any]
    ) -> None:
        """Refuse to run against an apparatus the round did not schedule.

        The sandbox mounts the live harness tree and the built `move-flow`
        binary, so editing either during a round silently changes what later
        cells measure. A run states the apparatus it was scheduled against, and
        a disagreement is a defect in the round rather than a result.
        """
        expected_harness = self.run.spec.controller_harness_sha256
        if expected_harness is not None and harness_hash != expected_harness:
            raise ValueError(
                "controller harness changed since the round was scheduled: "
                f"expected {expected_harness}, mounted {harness_hash}"
            )
        expected_binary = self.run.spec.move_flow_sha256
        if expected_binary is not None:
            actual = move_flow["sha256"]
            if actual != expected_binary:
                raise ValueError(
                    "move-flow binary changed since the round was scheduled: "
                    f"expected {expected_binary}, mounted {actual}"
                )
        # The solvers are the third thing this run measures with, and the one
        # named by environment rather than mounted: `BOOGIE_EXE` and `Z3_EXE`
        # are resolved at launch, so a backend upgraded between scheduling and
        # execution changes what "proved" means while the harness and
        # `move-flow` digests both still agree.
        expected_stages = self.run.spec.stage_executables
        if expected_stages:
            actual_stages = tool_executables(self.config)
            changed = changed_stages(expected_stages, actual_stages)
            if changed:
                raise ValueError(
                    "stage executable(s) changed since the round was scheduled "
                    f"({', '.join(changed)}): a verdict from one toolchain "
                    "cannot be reported as a result from another"
                )

    def _feedback_level(self) -> str:
        """The apparatus this cell runs under.

        A round that schedules the feedback factor states the level per run; one
        that does not takes it from the experiment configuration.
        """
        return self.run.spec.feedback_level or self.config.feedback_level

    def _write_candidate_check(self) -> None:
        """Publish the task's acceptance criteria outside the agent's workspace.

        The agent-visible candidate check reads its baseline, target, editable
        paths, and required categories from this file, so the criteria cannot be
        relaxed by editing the workspace.
        """
        write_json(
            self.candidate_check,
            {
                "schema_version": 1,
                "baseline": str(self.baseline_package),
                "package": str(self.package),
                "target": self.run.spec.target,
                "allowed_edit_paths": list(self.run.spec.allowed_edit_paths),
                "required_contract_categories": list(
                    self.run.spec.required_contract_categories
                ),
                "timeout_seconds": self.config.operational_timeout_seconds,
                # The agent-facing check spends local computation to locate a
                # timeout; the judge's verdict does not depend on it.
                "attribute_timeouts": self._feedback_level() != "baseline",
                # Per-condition progress between attempts is the mechanism the
                # `progress` level adds; below it the check reports a verdict
                # without saying which obligations moved.
                "report_conditions": self._feedback_level() == "progress",
                # The baseline comparison and the edit scope are enforced, not
                # only reported: a candidate that changed the implementation it
                # was asked to specify is rejected. An ordinary session keeps
                # the report-only default.
                "enforce_edit_policy": True,
            },
        )

    def _move_flow_args(self) -> str:
        """Arguments the session's MCP server must run with.

        The generated plugin launches the server through
        `${MOVE_FLOW_ARGS:-mcp}`, and that expansion resolves against the
        *process* environment rather than the server's own `env` block. These
        arguments therefore have to reach the agent process, not just the
        runtime config file; putting them in only one of the two silently
        degrades the session to the bare `mcp` default.
        """
        tactic = ARM_TO_TACTIC[self.run.spec.arm]
        level = self._feedback_level()
        command = (
            f"mcp --inference-tactic {tactic} --evaluation-mode "
            f"--feedback-level {level}"
        )
        # The task criteria are the acceptance intervention. A baseline cell
        # keeps the check tool with the package's own defaults, so calling it
        # yields no treatment feedback; the audit expects exactly this
        # asymmetry.
        if level != "baseline":
            command += f" --candidate-check {self.candidate_check}"
        return command

    def _write_runtime_mcp(self) -> None:
        config = json.loads((self.plugin_dir / ".mcp.json").read_text(encoding="utf-8"))
        server = config["mcpServers"]["move-flow"]
        environment = server.setdefault("env", {})
        environment["MOVE_FLOW_TELEMETRY_JSONL"] = str(self.flow_events)
        environment["MOVE_FLOW_ARGS"] = self._move_flow_args()
        binary = shutil.which("move-flow")
        if not binary:
            raise RuntimeError("move-flow is not available on PATH")
        environment["MOVE_FLOW"] = str(Path(binary).resolve())
        if proxy := os.environ.get("MOVE_INFERENCE_BOOGIE_PROXY"):
            # Inside the sandbox the agent's prover reaches Boogie through the
            # controller's proxy; see `boogie_proxy.py` for why it cannot run
            # the executable itself.
            environment["BOOGIE_EXE"] = os.environ["MOVE_INFERENCE_BOOGIE_AGENT"]
            environment["MOVE_INFERENCE_BOOGIE_PROXY"] = proxy
        write_json(self.runtime_mcp, config)

    @asynccontextmanager
    async def _agent_boogie_proxy(self):
        """Serve Boogie to the agent for the lifetime of one session."""
        if self.agent_kind == "fake":
            yield
            return
        socket_value = os.environ.get("MOVE_INFERENCE_BOOGIE_PROXY")
        executable_value = os.environ.get("MOVE_INFERENCE_BOOGIE_REAL")
        if not socket_value or not executable_value:
            raise RuntimeError("pilot sandbox did not configure the Boogie proxy")
        async with BoogieProxy(
            Path(socket_value),
            Path(executable_value),
            (self.workspace_root, Path("/tmp")),
        ):
            yield

    def _make_agent(self, event_log: JsonlWriter, attempt: int = 1) -> AgentSession:
        if self.agent_kind == "fake":
            if self.fake_script is None:
                raise ValueError("--fake-script is required for --agent=fake")
            return FakeAgentSession(
                self.fake_script, self.package, event_log, self.flow_events, attempt
            )
        stderr_path = self.artifact_dir / "stderr.log"

        def stderr_sink(line: str) -> None:
            with stderr_path.open("a", encoding="utf-8") as stream:
                stream.write(line)
                if not line.endswith("\n"):
                    stream.write("\n")

        return ClaudeAgentSession(
            self.config,
            self.package,
            self.plugin_dir,
            self.runtime_mcp,
            event_log,
            stderr_sink,
            self.flow_events,
            self.candidate_check,
            self.baseline_package,
            self._move_flow_args(),
        )

    def _validate_runtime_identity(self, init: dict[str, Any] | None) -> None:
        if self.agent_kind == "fake":
            return
        if not init or not isinstance(init.get("system"), dict):
            raise RuntimeError("missing system/init identity data")
        system = init["system"]
        if system.get("model") != self.config.model:
            raise RuntimeError(
                f"model mismatch: expected {self.config.model}, got {system.get('model')}"
            )
        if system.get("claude_code_version") != self.config.claude_code_version:
            raise RuntimeError(
                "Claude Code version mismatch: "
                f"expected {self.config.claude_code_version}, got {system.get('claude_code_version')}"
            )
        servers = init.get("mcp_status", {}).get("mcpServers", [])
        flow = next((server for server in servers if server.get("name") == "move-flow"), None)
        if not flow or flow.get("status") != "connected":
            raise RuntimeError(f"move-flow MCP is not connected: {flow}")
        advertised = {tool.get("name") for tool in flow.get("tools", [])}
        expected = set(
            json.loads((self.plugin_dir / "move-flow-manifest.json").read_text(encoding="utf-8"))[
                "mcp_tools"
            ]
        )
        if advertised != expected:
            raise RuntimeError(
                f"MCP tool inventory mismatch: expected {sorted(expected)}, got {sorted(advertised)}"
            )

    def _validate_flow_configuration(self, offset: int) -> None:
        """Fail the run when the MCP server did not receive its configuration.

        The server reports the feedback level and candidate check it actually
        resolved. A flag lost between the controller and the server would
        otherwise present as a plausible default, and the arm would run without
        the apparatus it was scheduled for.
        """
        expected = self._feedback_level()
        for record in _read_jsonl_suffix(self.flow_events, offset):
            if record.get("event") != "session_start":
                continue
            level = record.get("feedback_level")
            if level is None:
                # A build predating this telemetry cannot be checked.
                return
            if level != expected:
                raise RuntimeError(
                    f"Flow feedback level mismatch: scheduled `{expected}`, "
                    f"server resolved `{level}`"
                )
            if expected != "baseline" and not record.get("candidate_check_configured"):
                raise RuntimeError(
                    f"Flow ran at `{expected}` without a candidate check; "
                    "the acceptance apparatus would be silently absent"
                )
            return

    def _flow_restart_error(self, offset: int) -> str | None:
        """Report a Flow MCP supervisor restart recorded after ``offset`` lines.

        A restarted MCP child loses its package cache and any tool call that was
        in flight, so the remainder of the session no longer measures the arm
        under its intended apparatus. Only telemetry written by the current
        attempt is considered, so a retry after a restart starts clean.
        """
        for record in _read_jsonl_suffix(self.flow_events, offset):
            if record.get("event") == "session_start" and record.get("restart") is True:
                session = record.get("session_id")
                return f"Flow MCP supervisor restarted session {session!r}"
        return None

    def _flow_infrastructure_error(self, offset: int) -> str | None:
        """Report a prover that could not run during the agent's turn.

        The in-session check and verify tools then told the agent nothing about
        its specification, so the turn measured the apparatus, not the arm. The
        eventual judge runs outside the agent's confinement and can still
        accept the tree, which is exactly how such a run would otherwise pass
        as a success.
        """
        for record in _read_jsonl_suffix(self.flow_events, offset):
            event = record.get("event")
            if event == "candidate_check" and record.get("state") == "infrastructure_failure":
                return "in-session candidate check could not run the prover"
            if event == "prover" and record.get("outcome") == "infrastructure":
                return "in-session verify could not run the prover"
        return None

    def _reset_workspace(self) -> None:
        if self.workspace_root.exists():
            shutil.rmtree(self.workspace_root)
        copy_snapshot(self.baseline_root, self.workspace_root)

    def _finalize(self, result: dict[str, Any]) -> None:
        (self.artifact_dir / "stdout.log").write_text(
            json.dumps(result, sort_keys=True) + "\n", encoding="utf-8"
        )
        final_root = self.artifact_dir / "final"
        copy_final_tree(self.package, final_root)
        (self.artifact_dir / "workspace.diff").write_text(
            workspace_diff(self.baseline_package, final_root), encoding="utf-8"
        )
        write_json(self.artifact_dir / "judge.json", result)
        run_path = self.artifact_dir / "run.json"
        run_record = json.loads(run_path.read_text(encoding="utf-8"))
        run_record["result"] = result
        write_json(run_path, run_record)
        redact_tree(self.artifact_dir)

    def _wall_seconds(self) -> float:
        return (time.monotonic_ns() - self.started_ns) / 1_000_000_000


def _count_lines(path: Path) -> int:
    if not path.is_file():
        return 0
    return len(path.read_text(encoding="utf-8").splitlines())


def _read_jsonl_suffix(path: Path, offset: int) -> list[dict[str, Any]]:
    """Parse the JSONL records after ``offset`` lines, skipping partial writes."""
    if not path.is_file():
        return []
    records: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines()[offset:]:
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            records.append(value)
    return records


def _binary_identity(name: str) -> dict[str, Any]:
    path = shutil.which(name)
    if not path:
        raise RuntimeError(f"required executable `{name}` is not on PATH")
    resolved = Path(path).resolve()
    version = subprocess.run(
        [str(resolved), "--version"],
        check=False,
        capture_output=True,
        text=True,
        timeout=10,
    )
    return {
        "path": str(resolved),
        "sha256": sha256_file(resolved),
        "version": (version.stdout or version.stderr).strip(),
        "version_exit_code": version.returncode,
    }


def _infrastructure_result(diagnostics: str) -> JudgeResult:
    return JudgeResult(
        schema_version=2,
        state="infrastructure_failure",
        diagnostics=diagnostics,
        verdict={},
        command={},
        tree_sha256="",
    )


def _agent_infrastructure_error(turn: AgentTurn) -> str | None:
    """Classify SDK/provider failures before judging an unchanged workspace.

    Claude Code reports some provider failures, notably missing authentication,
    with ``terminal_reason=api_error`` but without an HTTP status.  Treating
    those as normal model answers would spend all controller turns and record a
    benchmark failure instead of invalid infrastructure.
    """

    if not turn.is_error:
        return None
    if turn.api_error_status is not None:
        return f"provider API error {turn.api_error_status}"
    if turn.terminal_reason == "api_error":
        detail = turn.result.strip() or "provider API error without status"
        return f"provider API error: {detail}"
    return None


WALL_INTERRUPT_GRACE_SECONDS = 30.0


async def _send_with_wall_deadline(
    agent: AgentSession, prompt: str, remaining_seconds: float
) -> tuple[AgentTurn, bool]:
    """Enforce the run wall budget even while one SDK turn is in flight.

    A turn that outlives the budget is interrupted rather than cancelled: the
    session then still delivers its final result message, which carries the
    cumulative usage telemetry a cancellation would discard. Only when the
    interrupt itself stalls is the turn abandoned. Returns the turn and
    whether the deadline was hit.
    """
    if remaining_seconds <= 0:
        raise TimeoutError
    task = asyncio.ensure_future(agent.send(prompt))
    done, _ = await asyncio.wait({task}, timeout=remaining_seconds)
    if task in done:
        return task.result(), False
    try:
        await agent.interrupt()
        turn = await asyncio.wait_for(task, timeout=WALL_INTERRUPT_GRACE_SECONDS)
        return turn, True
    except Exception as error:
        task.cancel()
        with suppress(BaseException):
            await task
        if isinstance(error, TimeoutError):
            raise
        raise TimeoutError from error


def _task_mutants(root: Path | None, task_id: str, label: str = "mutant") -> Path | None:
    """The task's manifest under a mutant root, or `None` when unset."""
    if root is None:
        return None
    manifest = root.resolve() / task_id / "mutants.json"
    if not manifest.is_file():
        raise SystemExit(f"{label} manifest not found: {manifest}")
    # An empty set refutes nothing: it finds no survivor and no inconclusive
    # result, so the candidate is confirmed having been tested against
    # nothing. Asking for a refutation root and getting silence is the one
    # reading that must not be available.
    if not load_object(manifest).get("mutants"):
        raise SystemExit(f"{label} manifest lists no mutants: {manifest}")
    return manifest


def _require_disjoint_from_scoring(run_spec: Any, refutation: Path | None) -> None:
    """Refuse a refutation set that overlaps the one this round is scored on.

    A contract repaired against a mutant it was shown cannot then be measured
    by it: the round would report "the contract is complete" when what it
    observed was "the agent can act on feedback".

    Comparing the two root paths, or the two manifests' digests, catches only
    the exact repeat -- a copy at another path, or a manifest mixing one scored
    mutation in with others, defeats both while post-round scoring still
    accepts the original by its recorded digest. The scheduled per-mutation
    fingerprints are the relation that actually holds, and they subsume the
    weaker checks, so this is the only one. They are opaque, so the scored
    mutations still never exist anywhere the session can read.
    """
    if refutation is None or run_spec.spec.mutant_manifest_sha256 == NO_MUTANTS:
        return
    if not run_spec.spec.mutant_identities:
        # A scored manifest with no recorded identities cannot be checked, and
        # the scheduler does not produce that state -- so it means the round
        # was built by something older, not that there is nothing to compare.
        raise SystemExit(
            "run spec records a scoring manifest but no mutant identities, so an "
            "overlapping refutation set could not be detected; reschedule the "
            "round with a current pilot build"
        )
    overlap = overlapping_mutations(
        load_object(refutation)["mutants"],
        set(run_spec.spec.mutant_identities),
        run_spec.shared_package,
    )
    if overlap:
        raise SystemExit(
            f"{refutation} repeats mutation(s) this round is scored on "
            f"({', '.join(overlap)}): a contract repaired against a mutant it "
            "was shown cannot then be measured by it"
        )

def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--agent", choices=("claude", "fake"), default="claude")
    parser.add_argument("--fake-script", type=Path)
    parser.add_argument(
        "--refutation-mutants-root",
        type=Path,
        help="directory of TASK_ID/mutants.json used as feedback during the run: "
        "an accepted candidate that fails to reject one is sent back. Must not be "
        "the set the round is scored on -- see --hidden-mutants-root.",
    )
    parser.add_argument(
        "--hidden-mutants-root",
        type=Path,
        help="directory containing TASK_ID/mutants.json; never mount it in the agent sandbox",
    )
    parser.add_argument(
        "--skip-hidden-scoring",
        action="store_true",
        help="run without a hidden mutant set; strict_success stays false and "
        "strict scoring is left to `harness.score_round` after the round",
    )
    parser.add_argument(
        "--allow-unsandboxed",
        action="store_true",
        help="development only: bypass the real-agent OS-sandbox marker check",
    )
    args = parser.parse_args()
    if (
        args.agent == "claude"
        and not args.allow_unsandboxed
        and os.environ.get("MOVE_INFERENCE_EVAL_SANDBOXED") != "1"
    ):
        raise SystemExit(
            "real runs require an OS sandbox; set MOVE_INFERENCE_EVAL_SANDBOXED=1 inside the configured container"
        )
    config = ExperimentConfig.load(args.config.resolve())
    configured_endpoint = os.environ.get("ANTHROPIC_BASE_URL")
    if args.agent == "claude" and configured_endpoint not in (None, config.provider_base_url):
        raise SystemExit(
            f"provider endpoint mismatch: config pins {config.provider_base_url}, environment has {configured_endpoint}"
        )
    # The flag and the mutant set are alternatives, not decorations: asking for
    # both hides which one the run actually used.
    if args.skip_hidden_scoring and args.hidden_mutants_root is not None:
        raise SystemExit("--skip-hidden-scoring and --hidden-mutants-root are exclusive")
    run_spec = RunSpec.load(args.run.resolve()).resolve_paths(args.run.resolve())
    hidden_mutants = None
    if args.hidden_mutants_root is not None:
        if run_spec.spec.mutant_manifest_sha256 == NO_MUTANTS:
            raise SystemExit(
                "run has no mutant set; omit --hidden-mutants-root for core scoring"
            )
        hidden_mutants = _task_mutants(
            args.hidden_mutants_root, run_spec.spec.task_id, "hidden mutant"
        )
    refutation_mutants = _task_mutants(args.refutation_mutants_root, run_spec.spec.task_id)
    _require_disjoint_from_scoring(run_spec, refutation_mutants)
    base = Path(__file__).resolve().parent.parent
    controller = Controller(
        config,
        run_spec,
        args.artifacts.resolve() / run_spec.spec.run_id,
        base / "prompts",
        args.agent,
        args.fake_script.resolve() if args.fake_script else None,
        hidden_mutants,
        refutation_mutants,
    )
    result = asyncio.run(controller.run_experiment())
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
