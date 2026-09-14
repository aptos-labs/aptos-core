# bench_towers

Towers of Hanoi, ported from the Are We Fast Yet benchmark suite.

## What it stresses

Call frames. AWFY's published profile for Towers is 39.4M calls against 601
loop iterations, the highest call-to-loop ratio in the suite. Almost all the
work happens in `move_disks`, `move_top_disk`, `push_disk`, and `pop_disk_from`,
each of which is a few instructions wrapped in a call. What the benchmark
measures is the cost of entering and leaving a frame.

The secondary axis is vector access. Every disk read or write goes through
`vector::borrow` / `vector::borrow_mut` on the node arena, so the workload also
prices bounds checks and field offsets on a small, hot vector.

## Initialization

None. The package has no resources and no `init_module`. Publish it and call
`run`.

## Entry points

```move
/// Pure kernel. Returns the total number of disk moves over all rounds.
public fun bench_towers(disks: u64, iters: u64): u64

/// Transaction surface. Aborts with EWRONG_RESULT if the count differs.
public entry fun run(_s: &signer, disks: u64, iters: u64, expected: u64)
```

## Knobs

| Knob | Meaning | Suggested values |
| --- | --- | --- |
| `disks` | Disks moved per round. Work grows as `2^disks`. | 13 (the AWFY setting), 10-18 for a sweep |
| `iters` | Rounds. Each round rebuilds the tower from scratch. | 1 and up; scales the result linearly |

Both are runtime arguments, as is `expected`. No compile-time constant bounds
the kernel, so nothing here can be constant-folded away.

## Expected values

`bench_towers(disks, iters)` is exactly `iters * ((1 << disks) - 1)`.

| `disks` | `iters` | Result |
| --- | --- | --- |
| 13 | 1 | 8191 |
| 13 | 3 | 24573 |
| 3 | 1 | 7 |
| 16 | 1 | 65535 |
| 18 | 1 | 262143 |

8191 is AWFY's own `verifyResult` value. A port that does not produce it is
wrong.

Note the off-by-one carried over from the original: `disks` of 13 builds a
tower of *fourteen* disks, sizes 13 down to 0, and then moves the top thirteen.
The largest disk stays parked at the bottom of pile 0 for the whole run.

## Port notes

Move has no recursive structs, so the original's `TowersDisk` chain becomes an
index arena: `nodes: vector<Disk>` with `Disk { size, next }` and `NULL =
u64::MAX` as the terminator. `piles: vector<u64>` holds the three pile heads.

Keeping the `next` field, rather than collapsing each pile into a plain
`vector<u64>` stack, is deliberate. The `getNext` / `setNext` traffic is part of
what the benchmark measures, and a stack rewrite would delete it.

Both of the original's guards are kept in the measured path:

- `EBIG_DISK_ON_SMALL` in `push_disk`, when a larger disk would land on a
  smaller one.
- `EEMPTY_PILE` in `pop_disk_from`, when a pile is empty.

Neither fires for a correct run, but both cost a comparison per move.

`move_disks` stays recursive, matching the original. Recursion depth is `disks`,
well inside the VM's call stack limit.

One deviation: `bench_towers` skips `move_disks` when `disks` is 0. The original
would recurse forever there; skipping keeps the `(1 << disks) - 1` identity true
across the whole knob domain instead of aborting on a `u64` underflow.

## Tests

```bash
cargo run -q -p aptos -- move test \
  --package-dir third_party/move/mono-move/benchmarks/bench_towers \
  --skip-fetch-latest-git-deps
```

The Homebrew `aptos` CLI is older than this repo's `move-stdlib` and cannot
compile it. Use the CLI built from this tree.

## Provenance

Ported from `benchmarks/Java/src/Towers.java` in
[`smarr/are-we-fast-yet`](https://github.com/smarr/are-we-fast-yet), which is
itself based on the SOM class library. MIT licensed; the notice is reproduced at
the top of `sources/towers.move`.

The workload characterization comes from AWFY's published `docs/metrics.md` and
from *Cross-Language Compiler Benchmarking: Are We Fast Yet?* (DLS'16), not from
runs in this repo.
