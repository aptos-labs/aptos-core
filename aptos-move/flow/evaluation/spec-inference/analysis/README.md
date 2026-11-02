# Analysis

The preregistered estimands and task-cluster bootstrap are specified in
[`DESIGN.md`](../DESIGN.md) section 6.

[`codex_round_report.py`](codex_round_report.py) is the analysis of a Codex
round, fixed before the round runs. It prices every recorded model request
with [`pricing.json`](pricing.json) and writes `REPORT.md`, `analysis.json`,
`cells.csv`, `requests.csv` and `pricing.json`, which `harness.publication`
admits into a result archive:

```text
python3 -m analysis.codex_round_report --round-dir evaluation-artifacts/ROUND
```

The long-context rate applies to a single request whose own input exceeds the
threshold, never to a sum over requests; reasoning tokens are part of the
output and billed once. [`../corpus-v4/REPRODUCE.md`](../corpus-v4/REPRODUCE.md)
describes the whole procedure.

Pilot notes (development rounds, never main-arm results):

- [`pilot-merge-005-partition.md`](pilot-merge-005-partition.md) — three arms
  on the Lomuto partition task, with the agents' final reports.
- [`compact-flow-live-pairs.md`](compact-flow-live-pairs.md) — two-replica AO/HG
  diagnostics on QP and BA after compact query, WP, and status changes.
