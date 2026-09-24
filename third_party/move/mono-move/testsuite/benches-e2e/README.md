# End-to-end Performance Workloads

Application-shaped Move packages for the MonoMove end-to-end performance harness (`../e2e-perf/`). Each one publishes a single package and runs a weighted mix of its entry points, so it is representative of the real protocol's traffic.

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
| `dex_aggregator` | `dex-aggregator` | Panora | Monomorphization at scale. Routes carry up to 32 type parameters across four pool backends, which is the verifier's limit and far past anything else in the suite. |
| `nft_mint_market` | `nft-mint-market` | Topaz | Resource groups at derived addresses. Token v2 mint, list, offer, and buy, with a three-way fee split per sale. |

## Constraints on a package here

1. **One address per package.** `publish_util.rs` asserts every module in a package declares the same address, then rewrites `0xB0` to a generated publisher address. Declare `bench = "0xB0"` and put every module under it.

2. **No cross-package dependencies.** One package is published per workload. Anything a workload needs lives inside it. Module names are globally unique across the suite (each is prefixed with its package's short tag) because the whole suite shares one address. Framework packages are the exception — they are already on chain, so `AptosFramework` and `AptosTokenObjects` are fine.

3. **No aborts, discards, or retries.** Every `bench_*` entry point must be self-healing: clamp instead of asserting, faucet when short, no-op when there is nothing to do. Admin-only `initialize` and `seed_*` functions may assert on the caller, since they run once from a known signer.

   The harness deliberately passes none of `--allow-aborts` / `--allow-discards` / `--allow-retries`, and that strictness is load-bearing twice over. An aborting transaction pays for a prologue and an abort, not for the swap or the liquidation the mix believes it is measuring, so a workload that drifts into aborting keeps reporting a number that no longer describes the work named in the table above. And a payload MonoMove cannot execute aborts on every attempt, which would otherwise read as a large speedup rather than as the gap it is.

4. **Bulk seeding is chunked.** A whole order book or reserve set in one transaction runs past the per-transaction execution limit. `clob_avl` chunks at 32 orders per side.

5. **The mix must balance.** Whatever state the workload measures — book depth, listing count, liquidation candidates, pool imbalance, queue length — the branches that produce it and the branches that consume it have to balance, or the state runs to a boundary and the measured branch stops doing work. A mix stage runs a million transactions; a drift of one unit per transaction against a seeded few thousand is gone inside the first 1% of the run, and what the remaining 99% measures is an empty tree, an unfilled buy, or a read-only walk. Seeding more does not fix it, it only moves the cliff.

   Drift the other way is just as bad, and easier to miss because nothing stops working. State that only grows deepens the JMT for the length of the run, so throughput falls inside the run and the median depends on how many blocks were recorded. `bridge_relay` keys both payload slots and DVN attestations by nonce and nonces never repeat, so both grew without bound until they were folded onto a ring. Check every table a nonce stream touches, not just the obvious one.

   Work out the equilibrium arithmetically before running anything: production rate per transaction, consumption rate per transaction, and the level where they meet. Then write the test that pins it, because the next person to retune a weight will not redo the arithmetic.

## Adding a workload

In order.

1. Write the package under `benches-e2e/<pkg>/`, with `tests/<pkg>_tests.move` covering the math kernel, one `test_<kind>_tolerates_<edge>` per self-healing branch, a test pinning the equilibrium from constraint 5, and a `test_harness_onboard_then_mix` that replays exactly what the Rust generator issues. The last two are what catch an argument mismatch and a collapsed mix before a three-hour CI run does.

2. Add a `#[test] fn test_<pkg>()` to `../tests/benches_e2e_unit_tests.rs`, which runs the package on both VMs.

3. Add a module to `crates/transaction-workloads-lib/src/bench_workflows.rs` with `Config`, `Onboard`, and `mix_worker`, plus a `BenchWorkflowKind` variant carrying the knobs.

   Constraint 5's equilibrium test goes wherever the rates live. When the rates come from the `MIX` weights and the per-transaction constants, that is a Rust `#[test]` here and it does not need to run Move. When they come from what an entry point does to state, it is a Move test in step 1.

4. Add a `TransactionTypeArg` variant in `crates/transaction-workloads-lib/src/args.rs`. Clap kebab-cases it, so `MyWorkload` becomes `--transaction-type my-workload`.

5. Regenerate the prebuilt bundle:
   ```bash
   ./testsuite/benchmark-workloads/generate.py
   ```
   Commit `crates/transaction-workloads-lib/prebuilt.mpb`. Nothing rebuilds it at `cargo build` time, so a stale bundle serves old bytecode. The `prebuilt-workloads-check` workflow rebuilds it on every PR that touches a source it is built from and fails if the committed copy differs.

6. Add a `Workload(...)` entry to `../e2e-perf/run_e2e_perf_test.py` at `block_size=500`, `blocking=False`.

7. Run it small before running it for real:
   ```bash
   REPEATS=1 NUM_BLOCKS_PER_TEST=3 NUM_INIT_ACCOUNTS=20000 \
     ONLY_WORKLOADS=<workload> \
     python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
   ```

8. Calibrate. The workload reports `uncalibrated` until it has a row in `../e2e-perf/e2e_perf_speedup.tsv`, which needs at least five CI runs. See `../e2e-perf/README.md`.

Attribution for a package's design lives in its Move source headers, not here.
