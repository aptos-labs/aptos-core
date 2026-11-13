# Verification benchmark problems

The problems the nightly benchmark measures, and its open decisions. The
benchmark itself — measurements, components, history, reports, CI and local
use — is [`designs/benchmark.md`](../designs/benchmark.md). The manifest is
`bench/problems.toml` (under `third_party/move/lean`).

## Problems

The problems are chosen to cover the verifier's cost centers: arithmetic,
loops, vectors, generics, storage, intrinsic maps, cross-module calls,
crypto natives, and the kernel-heavy unit preparation. The initial set:

| Problem | Input | Covers |
|---|---|---|
| `option`, `vector`, `string`, `error` | `move-stdlib` | small modules, vector reasoning, fixed per-problem cost |
| `bit_vector` | `move-stdlib` | loops (`shift_left` about 284M heartbeats) |
| `fixed_point32` | `move-stdlib` | non-linear arithmetic |
| `features` | `move-stdlib` | bit flags over vectors (`apply_diff` about 714M) |
| `acl`, `cmp` | `move-stdlib` | vector membership, native model of `compare` |
| `math64`, `math128` | `aptos-stdlib` | arithmetic, `sqrt` (about 53 s) |
| `math_fixed`, `fixed_point64` | `aptos-stdlib` | fixed-point arithmetic |
| `simple_map` | `aptos-stdlib` | intrinsic map, insertion order |
| `comparator`, `capability` | `aptos-stdlib` | generics, capability delegation |
| `pool_u64` | `aptos-stdlib` | quantified map invariants; `deduct_shares` is not verified (`pragma verex_verify = false`, roadmap) |
| `ristretto255` | `aptos-stdlib` | crypto natives, frame certificates (about 95 s) |
| `ed25519` | `aptos-stdlib` | crypto natives |
| `ordered_map` | `aptos-framework` | intrinsic map, sorted, iterators (about 80–110 s) |
| `storage` | `bench/package` | the typed memory: publish, take, frames, two calls in a row, `Coin<A>` beside `Coin<B>`, generic callers, stored data invariants |
| `behavior` | `bench/package` | behavioral predicates: `aborts_of`, `ensures_of`, `result_of`, `requires_of` over lambdas and named functions |
| `closures`, `closure_loop` | `bench/package` | function values: captures, struct fields, higher-order functions with bodies, invocation in a loop |
| `calculator`, `amm` | `higher-order-paper-26/examples` | the higher-order paper's examples: an enum state holding a continuation closure, and `amm`, a pool whose pricing closure is stored data, with its intended rejection |
| `Quicksort`, `OrderedMap` | `Check/Examples` | VerexSL: nested loops, authored proofs |
| `VectorOperations`, `GenericStorage` | `Check/Vectors`, `Check/Generics` | shared terms, generic storage |
| `bounds` | `SourceVerify` | the Rust frontend |

These are 31 problems; `type_info` is excluded pending its native model.
The `bench/package` problems are written for the
benchmark, to cover what the in-tree packages do not exercise. A problem that does not verify completely today
stays in the set: its curve then also shows the progress on it. Problems
that need table intrinsics (`smart_table`, `big_ordered_map`) join once
those intrinsics are supported. The higher-order paper's examples are the
exception: they are in the set before the state labels they need are
supported, so that their curves show that work.

## Open decisions

- **Noise.** The wall-time noise of the dedicated runner over repeated runs
  of one commit is not measured; that measurement sets the repeat counts and
  the report's threshold (10 % until then).
- **Runner queue.** The dedicated runner is shared with the execution
  performance and mono-move benchmarks, so the nightly may wait for them.
  If the wait becomes a problem, PIES can dispatch the benchmark at a time
  the others do not run.
- **Window.** 30 runs by default.
