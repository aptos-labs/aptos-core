# Verex Benchmark Reference Runs

Two reference JSON files for cost and completeness tracking (see `doc/plan/plain-values.md`).

## 2026-11-03-reference.json

- **Run:** 41770462997 (`gh run download 41770462997 --repo aptos-labs/aptos-core -n verex-bench-results`)
- **Commit:** 73672a3e3c70, branch `wrwg/verex-fx`, 2026-11-04T04:28:56Z
- **Results:** 30 verified, 1 failed (`amm`), 31 problems total
- **Role:** Cost and completeness reference for `doc/plan/plain-values.md` (before certified carriers)

## 2026-11-10-carriers.json

- **Run:** 42910622143 (`gh run download 42910622143 --repo aptos-labs/aptos-core -n verex-bench-results`)
- **Commit:** c76de5271aee, branch `wrwg/verex-fx`, 2026-11-10T22:35:06Z
- **Results:** 29 verified, 1 expected rejection (`amm`), 1 timeout (`OrderedMap`), 31 problems total
- **Role:** Post-certified-carriers baseline for regression comparison
