# MonoMove benchmark packages

Move packages used to measure MonoMove against the legacy MoveVM. Each package
is standalone: `Move.toml`, `sources/`, Move unit tests, and a `README.md` with
the initialization recipe, the entry points to call, and the knobs.

The rationale for picking these workloads is in
[`../docs/benchmark_candidates.md`](../docs/benchmark_candidates.md).

## Packages

| Package | Workload | What it stresses |
| --- | --- | --- |
| [`bench_towers`](bench_towers) | Towers of Hanoi (AWFY) | Call frames. Highest call-to-loop ratio in AWFY. |
| [`bench_sieve`](bench_sieve) | Sieve of Eratosthenes (AWFY) | Vector writes. The only write-dominated AWFY kernel. |
| [`bench_queens`](bench_queens) | N-queens backtracking (AWFY) | Read-heavy vectors, highly biased branches. |
| [`bench_bounce`](bench_bounce) | Bouncing balls (AWFY) | Field reads on a vector of structs. |
| [`bench_exchange2`](bench_exchange2) | Recursive sudoku enumeration | Deep recursion with backtracking. |
| [`bench_pathtracer`](bench_pathtracer) | Fixed-point path tracer | Signed 128/256-bit arithmetic, recursion, branchy control flow. |
| [`bench_aave`](bench_aave) | Lending market | Per-user read sets, resource groups, fungible assets. |
| [`bench_clob`](bench_clob) | Order book over a bit-packed AVL queue | Dependent table reads, shift-and-mask decoding. |
| [`bench_clmm`](bench_clmm) | Concentrated liquidity DEX | Read sets discovered by computation as a swap crosses ticks. |

All nine publish under the named address `bench = "0xB0"` and use globally
unique module names, so the whole suite can live at one address.

## Shared conventions

The six compute packages expose a pure kernel and a transaction wrapper:

```move
/// Pure kernel. Primitive arguments, one primitive return.
public fun bench_<name>(<knobs>): u64

/// Transaction surface. Aborts if the result does not match.
public entry fun run(_s: &signer, <knobs>, expected: u64)
```

The three DeFi packages expose per-user entry functions instead, plus a bulk
`seed_*` entry function for setup. `bench_clob` also has a `run` over its
`index_orders` kernel; `bench_aave` and `bench_clmm` have no single kernel to
wrap, so they have no `run`.

Every knob is a runtime argument, including `expected`. No compile-time constant
may let the optimizer fold a kernel away.

Each package README carries the initialization recipe, the entry-point mix, the
knobs and their suggested values, and the reference or golden values its tests
pin.

## Running the tests

```bash
cargo run -q -p aptos -- move test \
  --package-dir third_party/move/mono-move/benchmarks/<pkg> \
  --skip-fetch-latest-git-deps
```

The Homebrew `aptos` CLI is older than this repo's `move-stdlib` and cannot
compile it: it rejects `proof` blocks in `fixed_point32.move`, and at
`--language-version 2.4` it fails on `folds_of` and `unchanged_of` in
`vector.move`. Use the CLI built from this tree.

### Gas ceiling on unit tests

Every test is bounded at 1e9 internal gas units, and the bound cannot be raised.
`move-unit-test` defaults to `DEFAULT_EXECUTION_BOUND = 1_000_000`
(`tools/move-unit-test/src/lib.rs`), which is external `Gas`. `GasStatus::new`
converts it with `MULTIPLIER = 1000`
(`move-vm/test-utils/src/gas_schedule.rs:45`). The `aptos move test
-i/--instructions` flag looks like it adjusts this, but
`instruction_execution_bound` is declared and never read
(`aptos-move/cli/src/commands.rs:547`), so `gas_limit: None` always reaches the
default.

Do not read 1e9 as a billion bytecode instructions. `unit_cost_table` sets every
opcode to `GasCost::new(1, 1)`, and `GasCost::total()` is
`instruction_gas + memory_gas`, so a plain instruction costs 2. Size-weighted
opcodes cost more: `charge_instr_with_size` multiplies that total by
`AbstractMemorySize`. Measured on `bench_exchange2`, the real rate is 20 to 25
internal units per bytecode instruction, which puts the ceiling in the tens of
millions of instructions. Budget against measurement, not against the constant.

Exhausting the budget is reported as `Test timed out`, which is gas exhaustion
and not wall clock (`move-unit-test/src/test_runner.rs`).

Knob values that exceed the bound belong in the harness, not in a test. Each
package README marks which of its values those are.

## Provenance

The four AWFY kernels are ports of MIT-licensed source and carry the MIT notice.
`bench_aave` is reduced from Apache-2.0 `aave/aptos-aave-v3` with attribution.
`bench_exchange2`, `bench_pathtracer`, `bench_clob`, and `bench_clmm` are
written from published algorithm and data-structure descriptions; no code was
copied from a source under an incompatible license. Each package README states
what it was modeled on.
