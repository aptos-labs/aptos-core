# Plain values: open exceptions to the completeness gate

The plain-values plan ([`plain-values.md`](../plan/plain-values.md), "Gates")
asks that the benchmark's problems and the framework sample verify at least
what the reference of 2026-11-03 verified. This file lists the targets that
do not, with what is known of each. A cause is stated only where a trace
established it; "unknown" means no trace was made.

The counts against the reference: 26 of the 30 reference-verified benchmark
problems verify, and 353 of the 382 reference-verified targets of the
framework sample (24 modules, 588 targets). The Move Prover's lean gate, the
four suites, the e2e sweeps and prover-smoke are green.

## Benchmark problems

| Target | Observed | Cause |
|---|---|---|
| `simple_map::add_all` | 4 unproved leaves | A split of a match leaves a pattern equation that is not in normal form: the reading produced for the split's value (`readingsAtValue?`, `Solve/Base.lean`) is not reduced at the constructor. Deferred to the producer work of [`one-spelling.md`](../plan/one-spelling.md). |
| `calculator::process` | 19 unproved leaves | The same, for a match of a pair followed by a closure invoke. Deferred with the row above. |
| `pool_u64::buy_in` | maximum recursion depth in the closer | The failure is reached before the closer's alternatives run. The overflowing step is unknown. |
| `ordered_map::test_verify_iter_sum_symbolic` | a rewrite of the authored proof fails, then the enclosing rejection | Unknown. |
| `amm::constant_product_with_fee_non_compliant` | 5 unproved leaves and the enclosing rejection | Unknown. The manifest expects only `create_noncompliant_fee_pool` to be rejected. The reference rejected this target too; the run with the carriers verified it. |
| `rust_math64::lcm` | a proof artifact is missing and the kernel reports an unknown constant | Unknown. The problem is not in the reference. |
| `rust_simple_map::borrow_mut`, `rust_simple_map::remove` | an out-of-scope local and an unproved step; maximum recursion depth | Unknown. The problem is not in the reference. |

## Framework sample

Of the targets the reference verified, 34 fail and 16 stop at the budget.

| Targets | Observed | Cause |
|---|---|---|
| `coin`, `fungible_asset`, `primary_fungible_store`: 17 targets with a join in a unit that declares a spec ghost | unproved join premises | The premises of a join are collected at the carrier family induced by an inlined generic callee, while the facts in scope are at the unit's own family. Deferred to the producer work of [`one-spelling.md`](../plan/one-spelling.md). |
| `jwks::remove_oidc_provider_internal`, `code::initialize` | the fact of a resource's field is not found | The context states the invariant of the whole resource and never the field's own fact. Not repaired. |
| A target whose owner is rebound by a split (`Prod.fst` applied to a scalar in a ghost projection) | malformed projection, rejected | `Contract.quotedRowProjections` receives an owner type and a value that do not belong together. Where the pairing first goes wrong is unknown. |
| `big_ordered_map`: 10 targets, among them `test_verify_iter_write_loop_symbolic` (an unproved step of a loop invariant) | unproved leaves | Unknown. |
| `staking_contract`, `staking_config` and the remaining single targets | unproved leaves | Unknown. |
| 16 targets | stop at the 300M budget | Not attributed. Cost against the reference is 1.11 on the targets verified in both. |

## Other

A lemma of a pure specification function over a map loses a fact that its
authored proof states (`Verify/Steps.lean`, `giveFact`). The same happens
before the plain-values switch. The rule that discharges the fact is unknown.
