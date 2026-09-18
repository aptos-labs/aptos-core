# bench_exchange2

Recursive sudoku enumeration over three `vector<u64>` bitmask arrays, shaped
like SPEC CPU2017's 548.exchange2.

## What it stresses

Deep recursion with backtracking. `solve` calls itself once per cell, so a run
reaches 82 nested frames and stays there for most of its life. On the way down
it sets one bit in each of three shared masks; on the way up it restores all
three. That set-and-restore pair around a recursive call is the whole workload.

The rest of the mono-move suite does not cover this. `bench_towers` is
call-dominated but its state is a stack of pegs, not shared mutable state that
every frame writes and unwinds. `bench_queens` backtracks but stops at the first
solution, so its tree stays shallow and narrow. The existing `fib` fixture
recurses shallow and wide.

The working set is 27 `u64` masks plus 81 bytes of grid, so it fits in L1 many
times over. What is being measured is frame setup, the branch on each candidate
digit, and six vector writes per node, not memory traffic.

Two choices keep the tree large on purpose:

- Cells are visited in a fixed row-major order, not by minimum remaining values.
  An MRV heuristic prunes most of the tree and would shrink the benchmark by
  orders of magnitude.
- Every completion is counted. The search never stops at the first solution, so
  a grid with many solutions does all the work its constraints allow.

## Initialization

None. `bench_exchange2` builds its own masks and `run` touches no global state,
so the package can be published and called immediately.

## Entry points

```move
public fun bench_exchange2(puzzle_id: u64, iters: u64): u64
public fun bench_exchange2_grid(grid: vector<u8>, iters: u64): u64
public fun count_solutions(grid: &vector<u8>): u64
public entry fun run(_s: &signer, puzzle_id: u64, iters: u64, expected: u64)
```

`bench_exchange2` picks a grid from the built-in table and enumerates it `iters`
times, returning the summed solution count. `bench_exchange2_grid` takes the
grid directly: 81 bytes, row-major, `0` for an empty cell. It is the surface to
use from a transaction, since 81 four-bit cells do not fit in a `u256`.
`count_solutions` is one enumeration of one grid.

`run` calls `bench_exchange2` and aborts with `EBAD_RESULT` (1) if the result
differs from `expected`.

Every knob, `expected` included, is a runtime argument, so no constant lets the
optimizer fold the kernel away. The built-in grids are constants, but
`puzzle_id` is not, so the compiler cannot know which one the search runs on.

Abort codes: `EBAD_RESULT` (1), `EBAD_PUZZLE_ID` (2) for a `puzzle_id` at or
above 6, `EBAD_GRID` (3) for a grid that is not 81 cells or holds a value above
9. A grid whose givens already conflict is not an error; it returns 0.

## Knobs

| Knob | Meaning | Notes |
| --- | --- | --- |
| `puzzle_id` | Which built-in grid, 0 to 5 | The real cost knob. Spans four orders of magnitude. |
| `iters` | Repeats of the whole enumeration | Scales cost linearly and exactly. |

The table is a cost ladder. Each grid is roughly an order of magnitude past the
one before it, so a harness can pick a size rather than dialing `iters` up on a
tiny kernel.

| `puzzle_id` | Givens | Solutions | Branch nodes |
| --- | --- | --- | --- |
| 0 | 30 | 1 | 231 |
| 1 | 30 | 28 | 1,196 |
| 2 | 17 | 1 | 2,836 |
| 3 | 25 | 1 | 21,682 |
| 4 | 17 | 1 | 156,189 |
| 5 | 30 | 1032 | 1,599,860 |

A branch node is one `solve` call that lands on an empty cell and iterates the
nine candidate digits. Cells that are already filled cost a frame and a return.

Cost tracks the size of the search tree, which is not a smooth function of the
number of givens. Fewer givens means a deeper unconstrained run of cells, but
badly placed givens leave a wide tree too. Puzzles 2 and 4 both have 17 givens,
the proven minimum for a unique solution, and differ in cost by 55x.

Recursion depth is 82 frames regardless of the grid, comfortably under Move's
1024-frame limit.

## Expected values

`bench_exchange2(puzzle_id, iters)` is `iters` times the solution count above.

| `puzzle_id` | `iters` | Result |
| --- | --- | --- |
| 0 | 1 | 1 |
| 1 | 1 | 28 |
| 1 | 2 | 56 |
| 2 | 1 | 1 |
| 3 | 1 | 1 |
| 4 | 1 | 1 |
| 5 | 1 | 1032 |

`iters = 0` returns 0 for any grid.

Every count in this table was cross-checked against an independent
minimum-remaining-values solver written for the purpose and then discarded. It
searches in a different order from the kernel here, so it cannot share a bug
with it. The two agreed on all six grids.

## Tests

```bash
cargo run -q -p aptos -- move test \
  --package-dir third_party/move/mono-move/benchmarks/bench_exchange2 \
  --skip-fetch-latest-git-deps
```

The Homebrew `aptos` CLI is older than this repo's `move-stdlib` and cannot
compile it. Use the CLI built from this tree.

Thirteen tests, about 8.5 seconds against a debug build of the CLI, most of it
puzzle 3.

The tests assert puzzles 0 to 3. Puzzles 4 and 5 are harness knobs only; their
counts are in the table above and were verified the same way as the rest.

What puts them there is the 1e9 internal gas units a unit test gets, derived in
[Gas ceiling on unit tests](../README.md#gas-ceiling-on-unit-tests). Nothing
raises it: the `--instructions` flag on `aptos move test` is declared at
`aptos-move/cli/src/commands.rs:547` and never read anywhere in the crate.

Bisecting on `iters` puts the ceiling between 173,456 and 216,820 branch nodes.
Puzzle 5 at 1,599,860 nodes is roughly ten times past it and fails with `Test
timed out`, which is how the runner reports gas exhaustion. Puzzle 4 at 156,189
nodes does fit, but by under 1.4x, and a margin that thin would break the test
on any change to the kernel's instruction mix. It also costs about 17 seconds on
its own against 2.3 for puzzle 3. Either reason alone would keep it out.

## Provenance

This is written from the published description of SPEC CPU2017's 548.exchange2,
a sudoku puzzle generator in Fortran by Michael Metcalf that SPEC characterizes
as recursive generate-and-test over a 9x9 grid. It is not a port. SPEC CPU2017
source is not redistributable and none of it was obtained, read, or consulted.

What is shared with 548.exchange2 is the shape: recursion one level per cell,
backtracking that mutates state on the way down and restores it on the way up,
a working set small enough to sit in L1, and a cost dominated by call frames and
branches. Everything else is this file's own.

Two things differ from the SPEC benchmark by design. 548.exchange2 generates
puzzles; this counts completions of a given grid, because a count is a single
primitive that a harness can assert on. And its cost knob is a table of grids
rather than a generator seed, so a given `puzzle_id` always does exactly the
same work.

The six built-in grids were produced locally. Puzzles 0, 1, 3 and 5 come from a
random complete grid with cells removed, keeping the solution count in a target
band. Puzzles 2 and 4 are row, column, band, stack and digit relabellings of a
published 17-clue puzzle; those transformations preserve both the clue count and
the solution count. The transform space was searched for one cheap enough to
assert in a unit test, which is puzzle 2, and one expensive enough to be worth
benchmarking, which is puzzle 4. Puzzle 2 is the cheapest of 20,000 sampled
transforms, which is why the ladder has no 17-given grid below it.
