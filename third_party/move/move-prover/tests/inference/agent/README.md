# Agent Recordings

This directory contains recordings of manual runs of the AI-powered
spec inference strategies, organized by subdirectory:

- `model-driven/` — recordings using the model-driven strategy (default)
- `agent-driven/` — recordings using the agent-driven strategy

Each experiment consists of:

- `<name>.move` — the original Move source (no specs)
- `<name>.enriched.move` — the final output with inferred+refined specs
- `<name>.log` — full log of the agent session (if recorded)

## Reproducing an Experiment

From the repository root:

```bash
MVC_LOG=move_prover::agent=debug cargo run -p move-prover -- \
    --language-version 2.4 \
    -d third_party/move/move-stdlib/sources \
    -a std=0x1 \
    -i --ai \
    <source.move> 2>&1 | tee <source.log>
```

Here the log setting will cause the tool to print messages inbetween agent and model to 
console.

For example, to reproduce the `loops` experiment in model-driven mode:

```bash
MVC_LOG=move_prover::agent=debug cargo run -p move-prover -- \
    --language-version 2.4 \
    -d third_party/move/move-stdlib/sources \
    -a std=0x1 \
    -i --ai=model-driven \
    third_party/move/move-prover/tests/inference/agent/model-driven/loops.move \
    2>&1 | tee loops.log
```

Use `--ai=agent-driven` instead of `--ai` to use the agent-driven strategy.

Requires `ANTHROPIC_API_KEY` to be set in the environment.

## Why These Are Not Automated Tests

Agent sessions are inherently non-deterministic. The generated enriched
specifications may differ between runs, including whether verification
succeeds at all. Because of this, baseline-based testing is not feasible
and results are recorded here for manual inspection instead.
