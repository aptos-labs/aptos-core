# bench_queens

N-queens backtracking over three `vector<bool>` occupancy maps, ported from Are
We Fast Yet.

## What it stresses

Read-heavy vector access with highly predictable branches. AWFY's published
dynamic profile for this kernel is 26.3M array reads against 8.15M writes, a
3.2:1 ratio, at a branch bias of 0.97. Those counts are for AWFY's own harness,
which runs far more rounds than the settings here; what carries over is the
ratio. It is the read-dominated complement to
[`bench_sieve`](../bench_sieve), which is write-dominated, and to
[`bench_towers`](../bench_towers), which is call-dominated.

The inner test is `free_rows[r] && free_maxs[c + r] && free_mins[c + n - r - 1]`
and short-circuits, so most iterations are one `borrow` and a taken branch. That
is close to a pure measurement of the vector read path under a branch predictor
that almost always guesses right.

## Initialization

None. `bench_queens` allocates its own board and `run` touches no global state,
so the package can be published and called immediately.

## Entry points

```move
public fun bench_queens(n: u64, iters: u64): u64
public entry fun run(_s: &signer, n: u64, iters: u64, expected: u64)
```

`bench_queens` solves the `n` queens problem from scratch `iters` times and
returns how many rounds found a solution. `run` calls it and aborts with
`EBAD_RESULT` (1) if the count differs from `expected`.

Every knob, `expected` included, is a runtime argument, so no constant lets the
optimizer fold the kernel away.

## Knobs

| Knob | Meaning | Notes |
| --- | --- | --- |
| `n` | Board size | 8 is the AWFY setting. Cost is the size of the search tree, not a smooth function of `n`. |
| `iters` | Rounds | Each round rebuilds the board, matching AWFY. Scales cost linearly. |

Cost grows with `n` but not monotonically, because the search stops at the first
solution and how deep that lies depends on the board. Calls to `place_queen`
for one round:

| `n` | 4 | 6 | 8 | 10 | 12 | 14 | 16 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| calls | 8 | 31 | 113 | 102 | 261 | 1899 | 10052 |

The AWFY setting `n = 8, iters = 10` is roughly 1.1K calls and 8.8K occupancy
tests, which is small for a transaction-level measurement. Raise `iters` to do
more work on the AWFY board, or raise `n` to grow the working set and the
recursion depth along with it.

## Expected values

`bench_queens(n, iters)` is `iters` when `n` queens can be placed and `0` when
they cannot. No solution exists for `n = 2` and `n = 3`; every other `n >= 1`
is solvable. `n = 0` counts as unsolved, since an empty board has no column to
place a queen in.

| `n` | `iters` | Result |
| --- | --- | --- |
| 8 | 10 | 10 |
| 8 | 1 | 1 |
| 6 | 1 | 1 |
| 1 | 1 | 1 |
| 2 | 1 | 0 |
| 3 | 1 | 0 |
| 0 | 1 | 0 |

`bench_queens(8, 10) == 10` is the AWFY setting: the Java runs ten rounds and
its `verifyResult` checks that every one of them solved.

A count of successful solves is a weak check, since any port that finds *some*
placement passes it. The test-only `first_solution(n)` returns the column chosen
for each row, which pins the search order as well as the result:

| `n` | `first_solution(n)` |
| --- | --- |
| 1 | `[0]` |
| 6 | `[3, 0, 4, 1, 5, 2]` |
| 8 | `[0, 6, 4, 7, 1, 3, 5, 2]` |

## Tests

```bash
cargo run -q -p aptos -- move test \
  --package-dir third_party/move/mono-move/benchmarks/bench_queens \
  --skip-fetch-latest-git-deps
```

The Homebrew `aptos` CLI is older than this repo's `move-stdlib` and cannot
compile it. Use the CLI built from this tree.

## Provenance

Ported from
[`Queens.java`](https://github.com/smarr/are-we-fast-yet/blob/master/benchmarks/Java/src/Queens.java)
in Are We Fast Yet, which derives from the SOM class library and is MIT
licensed. The notice is reproduced at the top of `sources/queens.move`.

The four occupancy arrays, the column-by-column recursion, and the search order
are unchanged from the Java. Three things differ.

The Java hardcodes 8 and sizes its arrays 8, 16, 16, 8. The port takes `n` at
runtime and sizes them `n`, `2n`, `2n`, `n`, keeping the Java's one slot of
slack over the `2n - 1` distinct diagonals.

The Java indexes the anti-diagonal as `freeMins[c - r + 7]`. On `u64` that
underflows and aborts whenever `c < r`, so the port computes `c + n - r - 1`
instead. It is the same number for every `r < n` and is never negative.

The Java folds the ten rounds with `&&`, which short-circuits and skips the
remaining rounds after the first failure. The port counts solved rounds instead,
so an unsolvable `n` still does all `iters` rounds of work.
