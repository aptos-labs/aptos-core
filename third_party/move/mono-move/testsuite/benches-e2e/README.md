# benches-e2e

Application-shaped Move packages for the MonoMove end-to-end performance harness (`../e2e-perf/`). Each one publishes a single package and runs a weighted mix of its entry points, so a number here moves for the same reasons a real protocol's would.

This is not `testsuite/benchmark-workloads/packages`. Those packages exist to stress one VM subsystem each — a loop, a vector, a table — and are deliberately synthetic. These are shaped after code that runs on mainnet: the control flow, the storage layout, and the transaction mix all come from a real protocol. A package here is slower to reason about and that is the point, because the parts of the VM that only show up under real application code are exactly the parts the synthetic workloads miss.

## Workloads

| package | `--transaction-type` | shaped after | what it stresses |
| --- | --- | --- | --- |
| `clob_avl` | `clob-avl` | Econia-style CLOB | Bit-packed AVL queue over table items. Order matching walks a tree whose depth depends on book state, so the read set is a pointer chase of data-dependent length. |
| `lending_market` | `lending-market` | Aave v3 | Supply, borrow, repay, and flash loans over eight reserves. Index accrual runs u256 math once per reserve per block, the same frequency it runs on chain, and a position touches several reserves at once. |
| `clmm_swap` | `clmm-swap` | Uniswap V3 | Concentrated liquidity. How densely the seeded pool initializes ticks decides how many a swap crosses — 15.8 per swap at the shipped density against 1.0 at the sparse one — so the read set widens from a few slots to dozens with no change in code path. |
| `stableswap` | `stableswap` | Curve | `get_D` and `get_y` Newton loops. Four pools spanning both amplification and coin count spread the iteration count from 1 to 10 rounds, so per-transaction compute cannot be constant-folded. |
| `bridge_relay` | `bridge-relay` | LayerZero V2 | The storage IO floor. Near-zero compute per message: a nonce compare, a payload write, an attestation. |
| `airdrop_fanout` | `airdrop-fanout` | AniAirdrop, Amnis campaign | Write fan-out. 100 recipients per transaction, mixing first-touch creation writes with repeat modification writes across sharded tables and primary fungible stores. |
| `oracle_batch` | `oracle-batch` | Switchboard | The native/interpreter boundary. Real ed25519 and secp256k1 verification over batched reports, with matched write-only and verify-only controls that isolate the native's share by subtraction. |
| `cdp_liquidation` | `cdp-liquidation` | Thala CDP, Liquity | Sequentially dependent resource reads. Each hop of the sorted vault list gets its next address only from the previous resource, so the reads cannot be issued in parallel. |
| `dex_aggregator` | `dex-aggregator` | Panora | Monomorphization at scale. Routes carry up to 32 type parameters across four pool backends, which is the verifier's limit and far past anything else in the suite. See the caveat below before reading its number. |
| `nft_mint_market` | `nft-mint-market` | Topaz | Resource groups at derived addresses. Token v2 mint, list, offer, and buy, with a three-way fee split per sale. |

### Caveat on `dex_aggregator`

The workload turns on `type_info::type_of<T>()` and `mode_bits<M>()` staying real work at runtime. Today they are: both VMs call a native, and `third_party/move/mono-move/natives/src/type_info.rs` carries a `TODO(completeness)` noting that the specializer could write the struct name at specialization time but does not.

If that TODO is ever resolved, `mode_bits<M>` folds to a constant on MonoMove and stays a native call on the legacy VM. This workload's reported speedup would then partly measure a fold only one side can perform, rather than the monomorphization cost it is named for. Nothing here breaks, and no test fails — the number just stops meaning what the table says it means. Recalibrate and re-read this row if that changes.

## Constraints on a package here

These come from the publishing and generation machinery, not from taste. Breaking any of them breaks the benchmark, usually as a panic three hours into a CI run.

1. **One address per package.** `publish_util.rs` asserts every module in a package declares the same address, then rewrites `0xB0` to a generated publisher address. Declare `bench = "0xB0"` and put every module under it.

2. **No cross-package dependencies.** One package is published per workload. Anything a workload needs lives inside it. Module names are globally unique across the suite (each is prefixed with its package's short tag) because the whole suite shares one address. Framework packages are the exception — they are already on chain, so `AptosFramework` and `AptosTokenObjects` are fine.

3. **No aborts, discards, or retries.** The harness passes none of `--allow-aborts` / `--allow-discards` / `--allow-retries`, so any of the three panics the run. Every `bench_*` entry point must be self-healing: clamp instead of asserting, faucet when short, no-op when there is nothing to do. Admin-only `initialize` and `seed_*` functions may assert on the caller, since they run once from a known signer.

4. **No publish, script, or multisig payloads in the measured transactions.** MonoMove discards them. Two-signer entry functions taking the user and the publisher are fine.

5. **Bulk seeding is chunked.** A whole order book or reserve set in one transaction runs past the per-transaction execution limit. `clob_avl` chunks at 32 orders per side.

   Nothing in this repo checks that a chunk size actually fits, so treat every one of them as an estimate. The unit-test runner does apply a ceiling — `DEFAULT_EXECUTION_BOUND * 1000` = 1e9 — but it meters with `unit_cost_table()`, which forces every instruction to cost 1 and prices every native at zero (`NativeGasParameters::zeros()`, both from the CLI and from `aptos_test_natives()`). A transaction on chain is held to `max_execution_gas` = 9.2e9 InternalGas on the production schedule, where natives are priced. Seeding is mostly native calls — objects, fungible assets, primary stores — so the term that would blow a chunk costs exactly nothing under the meter that runs in CI. Raising the unit-test bound does not close this; only executing against the production schedule would. Until then, an oversized chunk shows up as a failure in the e2e harness run, not as a failing test.

6. **No shared mutable state in the generator.** The transaction worker is a pure `Fn`. Anything per-account is recomputed from the account address with `slot_for`, never carried in a counter.

7. **The mix must balance.** Whatever state the workload measures — book depth, listing count, liquidation candidates, pool imbalance, queue length — the branches that produce it and the branches that consume it have to balance, or the state runs to a boundary and the measured branch stops doing work. A mix stage runs a million transactions; a drift of one unit per transaction against a seeded few thousand is gone inside the first 1% of the run, and what the remaining 99% measures is an empty tree, an unfilled buy, or a read-only walk. Seeding more does not fix it, it only moves the cliff.

   Drift the other way is just as bad, and easier to miss because nothing stops working. State that only grows deepens the JMT for the length of the run, so throughput falls inside the run and the median depends on how many blocks were recorded. `bridge_relay` keys payload slots by nonce and nonces never repeat, so it grew without bound until the slots were put on a ring.

   Work out the equilibrium arithmetically before running anything: production rate per transaction, consumption rate per transaction, and the level where they meet. Then write the test that pins it, because the next person to retune a weight will not redo the arithmetic. Six of the ten packages here shipped with this defect and were caught by review rather than by a failing test.

   This is also why a workload cannot rely on wall-clock time to renew anything. `executor-benchmark` advances the block timestamp by exactly one microsecond per block, so `timestamp::now_seconds()` does not move for an entire run.

## These numbers are single-threaded

`run_e2e_perf_test.py` passes `--execution-threads 1`, which takes the sequential execution path. Nothing here measures Block-STM.

Worth stating plainly, because it is easy to review a package and conclude otherwise. A singleton resource at `@bench` that every mix branch writes — a registry, a fee recipient, a price — looks like a write-write conflict that would serialize the workload and make it measure conflict handling rather than the subsystem it targets. Under Block-STM it would be. At one thread there is no conflict to abort on, and a hot slot costs a cache hit.

The consequence runs the other way, though: do not read a number here as a parallel-execution result, and do not assume a workload would scale if the harness were changed to run it on many threads. Several would not, for exactly the reason above. If you raise `--execution-threads`, the contention is real and the packages need revisiting first.

## Adding a workload

In order. Each step is cheap relative to the next.

1. Write the package under `benches-e2e/<pkg>/`, with `tests/<pkg>_tests.move` covering the math kernel, one `test_<kind>_tolerates_<edge>` per self-healing branch, a test pinning the equilibrium from constraint 7, and a `test_harness_onboard_then_mix` that replays exactly what the Rust generator issues. The last two are what catch an argument mismatch and a collapsed mix before a three-hour CI run does.

2. Add a `#[test] fn test_<pkg>()` to `../tests/benches_e2e_unit_tests.rs`, which runs the package on both VMs.

3. Add a module to `crates/transaction-workloads-lib/src/bench_workflows.rs` with `Config`, `Onboard`, and `mix_worker`, plus a `BenchWorkflowKind` variant carrying the knobs.

   Constraint 7's equilibrium test goes wherever the rates live. When the rates come from the `MIX` weights and the per-transaction constants, that is a Rust `#[test]` here and it does not need to run Move. When they come from what an entry point does to state, it is a Move test in step 1.

4. Add a `TransactionTypeArg` variant in `crates/transaction-workloads-lib/src/args.rs`. Clap kebab-cases it, so `MyWorkload` becomes `--transaction-type my-workload`.

5. Regenerate the prebuilt bundle:
   ```bash
   ./testsuite/benchmark-workloads/generate.py
   ```
   Commit `crates/transaction-workloads-lib/prebuilt.mpb`. Nothing rebuilds it at `cargo build` time and no CI check verifies it against the sources, so a stale bundle silently serves old bytecode.

6. Add a `Workload(...)` entry to `../e2e-perf/run_e2e_perf_test.py` at `block_size=500`, `blocking=False`.

7. Run it small before running it for real:
   ```bash
   REPEATS=1 NUM_BLOCKS_PER_TEST=3 NUM_INIT_ACCOUNTS=20000 \
     ONLY_WORKLOADS=<workload> \
     python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
   ```

8. Calibrate. The workload reports `uncalibrated` until it has a row in `../e2e-perf/e2e_perf_speedup.tsv`, which needs at least five CI runs. See `../e2e-perf/README.md`.

Attribution for a package's design lives in its Move source headers, not here.
