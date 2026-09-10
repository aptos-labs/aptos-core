# bench_bounce

A port of the Are We Fast Yet (AWFY) `Bounce` benchmark. `ball_count` balls with random positions and velocities are stepped `steps` times inside a 500x500 box, clamping to the walls and flipping velocity on contact. The kernel returns the total number of wall contacts.

Balls live in a `vector<Ball>` where `Ball` is a four-field struct of `i64`. Every step reads all four fields through `vector::borrow_mut` and writes back the ones the clamps touch. That is the point: `Bounce` has the highest field-read density in AWFY, 63.3M field reads against 17.7M calls, so it measures how well a VM gets a field out of a struct held in a vector.

## Initialization

None. The package publishes and runs. There is no global state, no `init_module`, and no seeding entry function.

## Entry points

```move
/// Total wall contacts over `iters` rounds.
public fun bench_bounce(ball_count: u64, steps: u64, iters: u64): u64

/// Transaction surface. Aborts with `EWRONG_RESULT` (1) if the result differs.
public entry fun run(_s: &signer, ball_count: u64, steps: u64, iters: u64, expected: u64)
```

Publish under `bench = 0xB0` and call:

```
0xB0::bounce::run(<signer>, 100, 50, 1, 1331)
```

## Knobs

| Knob | AWFY setting | Effect |
| --- | --- | --- |
| `ball_count` | 100 | Balls per round. Sets the size of the `vector<Ball>` and therefore the working set. |
| `steps` | 50 | Times each ball is advanced per round. |
| `iters` | 1 | Rounds. Each round rebuilds the balls from a fresh generator, so it multiplies the work and the result. |

Work is `ball_count * steps * iters` calls to `bounce`, plus `ball_count * iters` ball constructions.

Every knob is a runtime argument, `expected` included. Nothing in the kernel is a compile-time constant that would let an optimizer fold the loop away.

## Expected values

| Call | Result |
| --- | --- |
| `bench_bounce(100, 50, 1)` | 1331 |
| `bench_bounce(100, 50, 2)` | 2662 |
| `bench_bounce(100, 50, 10)` | 13310 |
| `bench_bounce(100, 50, 100)` | 133100 |
| `bench_bounce(100, 50, 1000)` | 1331000 |
| `bench_bounce(1000, 50, 1)` | 12764 |
| `bench_bounce(100, 500, 1)` | 13304 |
| `bench_bounce(7, 13, 1)` | 21 |
| `bench_bounce(1, 10, 1)` | 2 |

1331 is AWFY's published reference value for `Bounce`. Only `iters` scales the result linearly. Raising `ball_count` draws further into the same random stream rather than repeating it, so `bench_bounce(1000, 50, 1)` is not ten times `bench_bounce(100, 50, 1)`.

## Reproducing the reference count

The count is fragile. All of the following have to hold:

- `som.Random` is seeded at 74755 and each draw is `seed = ((seed * 1309) + 13849) & 65535`, returning the new seed. The first nine draws are 22896, 34761, 34014, 39231, 52540, 41445, 1546, 5947, 65224. `test_random_first_nine` pins them.
- One generator builds every ball in a round. Ball `i` takes four consecutive draws in the order x, y, x_vel, y_vel. A fresh generator per ball gives a different answer.
- Positions are `next() % 500`, velocities are `(next() % 300) - 150`. Velocities go negative, which is why the ball fields are `i64`.
- A step counts as one bounce however many walls it hits. 95 of the 5000 ball-steps in the reference run hit an x wall and a y wall at once, so tallying each clamp separately gives 1426 instead of 1331.
- Each clamp both pins the coordinate to the wall and forces the velocity to `+|v|` or `-|v|`. Flipping the velocity without pinning the coordinate gives 1210.

The port keeps AWFY's clamp order, `x > 500`, `x < 0`, `y > 500`, `y < 0`, but the order does not affect the result: a coordinate cannot be both above 500 and below 0, and the x clamps never touch `y`. All 24 permutations give 1331.

The mask keeps every draw in [0, 65535], so the `%` operations stay in `u64` and never depend on the sign rule for signed remainder.

## Tests

```bash
cargo run -q -p aptos -- move test \
  --package-dir third_party/move/mono-move/benchmarks/bench_bounce \
  --skip-fetch-latest-git-deps
```

The Homebrew `aptos` CLI is older than this repo's `move-stdlib` and cannot compile it. Use the CLI built from this tree.

## Provenance

Ported from the MIT-licensed AWFY sources, which carry the SOM class library notice reproduced at the top of `sources/bounce.move`:

- <https://github.com/smarr/are-we-fast-yet/blob/master/benchmarks/Java/src/Bounce.java>
- <https://github.com/smarr/are-we-fast-yet/blob/master/benchmarks/Java/src/som/Random.java>

Deviations from the Java original: `ball_count`, `steps`, and `iters` are runtime arguments instead of the hardcoded 100 and 50; balls are values in a `vector` instead of heap objects in an array; `Math.abs` is spelled out as `abs`. The arithmetic and the ordering are unchanged.
