# Measurement debts

Regressions accepted without analysis and instruments still missing. Methods:
[`performance.md`](../designs/performance.md), "How to measure".

| Debt | Measurement | What to do |
|---|---|---|
| `CompositionPerformance` typed targets | +9–12% over their baseline at equal object counts, re-recorded unanalyzed; +6–7% at `b4b850ba92f` (joins), +8–11% at `d351a430357` | bisect from `a8d3319bebe` with the solver profile of `set_seven` |
| `StateLabelWitnesses` | `inc_under_cap_twice` 26.42M against 25.57M at `76fd853e21`; cap raised 25,000k → 27,000k | attribute to the stated-facts and early-substitution steps of `b4b850ba92f`, then lower the cap |
| Stage counters | no counts of distinct contexts, repeated preparation requests, formula nodes and visits in `bindStep`, `splitStep`, `solveLeaf` and the preparation stages | add them before comparing the preparation levers |
| Request attribution | the join scheduler's first preparation pass shares bounds and position facts; per-request attribution is missing | finer counters in `automatic preparation` |
| Native re-measurement | every lever in `performance.md` was measured on encoded leaves | re-measure the benchmark, `DenotePerformance` and the map modules on native leaves |
