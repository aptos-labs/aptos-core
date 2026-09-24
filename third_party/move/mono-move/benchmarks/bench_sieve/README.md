# bench_sieve

Sieve of Eratosthenes over a `vector<bool>`, ported from Are We Fast Yet.

## What it stresses

Vector writes. In the AWFY measurements this kernel does 48M array writes
against 15M reads and only 60K calls, which makes it the only write-dominated
benchmark in the suite and the exact complement of
[`bench_towers`](../bench_towers).

The inner marking loop is a tight `borrow_mut`-and-store over a flat
`vector<bool>`, so the numbers it produces are close to a pure measurement of
the vector write path.

Setup is a visible share of the writes. At `size = 5000` a round does 5000
`push_back` calls to build the flags and 11069 marking stores against 4999
flag reads. AWFY allocates and fills per invocation too, so the port keeps it.

## Initialization

None. `bench_sieve` allocates its own flags and `run` touches no global state,
so the package can be published and called immediately.

## Entry points

```move
public fun bench_sieve(size: u64, iters: u64): u64
public entry fun run(_s: &signer, size: u64, iters: u64, expected: u64)
```

`bench_sieve` sieves `[2, size]` from scratch `iters` times and returns the sum
of the per-round prime counts. `run` calls it and aborts with `EBAD_RESULT` (1)
if the sum differs from `expected`.

Every knob, `expected` included, is a runtime argument, so no constant lets the
optimizer fold the kernel away.

## Knobs

| Knob | Meaning | Notes |
| --- | --- | --- |
| `size` | Upper bound of the sieve | 5000 is the AWFY setting. Cost is roughly `size * ln ln size`. |
| `iters` | Rounds | Each round reallocates the flags, matching AWFY. Scales cost linearly. |

## Expected values

`bench_sieve(size, iters)` is `iters * pi(size)`, where `pi` counts the primes
up to and including `size`.

| `size` | `iters` | Result |
| --- | --- | --- |
| 5000 | 1 | 669 |
| 5000 | 3 | 2007 |
| 10000 | 1 | 1229 |
| 1000 | 1 | 168 |
| 100 | 1 | 25 |

669 at `size = 5000` is the value AWFY's own `verifyResult` checks.

The Java allocates `boolean[size]` and runs `i` from 2 to `size` inclusive while
reading `flags[i - 1]`, so index 0 is never inspected. The port keeps that
indexing. Since index 0 stands for the number 1, which is not prime, the count
is exactly `pi(size)` with no off-by-one to reconcile.

## Tests

```bash
cargo run -q -p aptos -- move test \
  --package-dir third_party/move/mono-move/benchmarks/bench_sieve \
  --skip-fetch-latest-git-deps
```

The Homebrew `aptos` CLI is older than this repo's `move-stdlib` and cannot
compile it. Use the CLI built from this tree.

## Provenance

Ported from
[`Sieve.java`](https://github.com/smarr/are-we-fast-yet/blob/master/benchmarks/Java/src/Sieve.java)
in Are We Fast Yet, which derives from the SOM class library and is MIT
licensed. The notice is reproduced at the top of `sources/sieve.move`.

The loop structure and bounds are unchanged from the Java. No wheel, no `sqrt`
bound, no skipping of even numbers: the writes are the workload, so making the
sieve smarter would defeat the measurement. The port adds only the `iters` and
`expected` knobs and the flag allocation the Java gets from
`new boolean[]` plus `Arrays.fill`.
