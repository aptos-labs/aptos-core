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
| Closure terms in leaves | `StateLabelCallObservations` `wrong_result` (a rejected false postcondition) 19.15M before native function values step 2a, 26.07M after (over its 25M budget); `grind` worked on the spelled-out `functionOf` closures and their `Sharing` literals and rows; abstracting function values before `grind` brings it to 21.31M | the representation fix: closures as constructor atoms (step 2b of `native-function-values.md`) and sharing marks as derived data of the target, then re-measure |
| Proof-language items | the L1 gate's per-item figures (`proof-language.md`, after "Milestones") were measured on single-target isolations (`scripts/isolate-target.py`), the scripts at the tree that held them; no module total of `ordered_map` with every target was recorded | record the module's native total with the items and with the scripts, both on one tree |
