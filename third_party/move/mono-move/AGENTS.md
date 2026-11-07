# mono-move

Conventions for every crate under `third_party/move/mono-move/`. Crate-specific
docs live in each crate's `AGENTS.md`; design rationale in `docs/`.

## Pull requests

Every PR that touches `third_party/move/mono-move/` carries the `mono-move`
label (`gh pr create --label mono-move`, or add it afterwards). The label is
what triggers the label-gated CI for this tree: the micro-benchmark
regression gate (`mono-move-micro-bench.yaml`) and the parity run
(`mono-move-tests-parity.yaml`) only run on labelled PRs, and the label is
how mono-move changes are found later. `mono-move-e2e-perf` is a separate,
opt-in label for the e2e perf workflow.

## Custom lints

`devtools/lints` holds repo-specific lints run with `cargo dylint`
(`scripts/dylint.sh` here runs them over every mono-move crate; CI does the
same on mono-move PRs). They are deny-by-default; a hit is a review finding.
Suppress one deliberately with
`#[cfg_attr(dylint_lib = "aptos_lints", allow(<lint>))]` plus a comment.
Current lints:

- `no_std_hash_map` — no `std`/`hashbrown` hash maps or sets in the
  consensus-critical crates; use `shared_dsa::{UnorderedMap, UnorderedSet}`
  (which expose no iteration, so nothing can depend on hash order) or
  `BTreeMap`/`BTreeSet` when ordered iteration is needed.
- `raw_memory_access` — no raw pointers, raw-pointer method calls, or
  `ptr::`/`slice::from_raw_parts` calls outside `mem` and `alloc`; see
  "Raw memory access".

## Raw memory access

All raw memory access goes through `MemAddr` in the `mono-move-mem` crate
(`mem/`). The VM-level types sit on top of it: `FatRef`, `Frame` and `Obj` in
`mono_move_core::memory`.

Rules:

- No crate other than `mem` and `alloc` holds a raw pointer (`*mut T` /
  `*const T`, any `T`), calls a method on one, or calls the `ptr::` /
  `slice::from_raw_parts` family. Raw pointers enter through
  `MemAddr::from_ptr` / `from_ref` / `from_typed_ptr` at allocator, FFI and
  code-pointer boundaries; there is no method that yields one back. A
  code-pointer word becomes a typed reference through
  `MemAddr::as_ref::<T>()`. Enforced by the `raw_memory_access` lint (see
  "Custom lints"; `lean-link`'s C ABI is exempt by config).
- Pointer-bearing words go through `load_ptr` / `store_ptr`, never
  `load::<u64>` (that strips provenance). Eight opaque bytes that may be a
  pointer, a scalar, or padding move with `copy_word`.
- Wrapper unsafety: constructing `Frame` / `Obj` is `unsafe` and asserts the
  pointee invariant once; methods that take an untrusted offset (`slot`,
  `at`, `field`, `vec_elem`, `FatRef::target`) are `unsafe`; what the
  construction covers (`meta()`, `header()`) is safe; every load and store
  is `unsafe`.
- Every `unsafe` block carries a `// SAFETY:` comment naming the concrete
  invariant that discharges the method's `# Safety` contract — which
  verifier check, which allocator guarantee, which null check above — not
  "caller must uphold".

## Memory checks

Runtime memory checks live at the choke point (`MemAddr`, `FatRef`, `Frame`,
`Obj`) and are attached with `mono_move_mem::mem_check!(<tier>, cond, fault)`.
A check only ever fails on a VM bug; it never changes what a valid program
computes, and it is never metered.

- **Tiers** are a build property of `mono-move-mem`, selected by
  `MONO_MOVE_MEM_CHECKS=hardened|exhaustive` when invoking cargo (unset =
  production, no check code compiled). Not a cargo feature on purpose:
  features are additive across the dependency graph and would let a tool
  crate silently harden the node. `mono_move_mem::CHECK_TIER` says what a
  binary was built with; the block executor logs it at startup.
- `hardened`: O(1) per access; the candidate for running on validators one
  day. `exhaustive`: anything heavier; CI and fuzzing only. Below its tier a
  `hardened` check is a `debug_assert!`; an `exhaustive` check is nothing.
- **Failure**: `mono_move_mem::checks::fault` records a `MemFault` for
  `take_last_fault()` and unwinds; harnesses map it to an invariant violation.
- **Adding a check**: attach it where the invariant is established, not at
  the call site; give it a `MemFaultKind`; add a negative test next to the
  type (the `assert_fault` pattern in `mem/src/addr.rs`), which is a no-op
  in the production tier; run the differential suite under both tiers
  (`.github/workflows/mono-move-mem-checks.yaml` does) — it must show zero
  faults on valid programs.
- `testsuite/benches/perf/run.sh` refuses to run with a tier set unless
  `MONO_MOVE_MEM_CHECKS_BENCH=1`, so the gate always measures production.

## TODO labels

Every inline TODO is `// TODO(<group>): message`, where `<group>` is one or more
(comma-separated, most relevant first) of:

| group | covers |
|---|---|
| correctness | wrong results, unsoundness, aliasing, legacy-VM parity |
| security | attacker-reachable abuse: unbounded user-controlled input, cache pollution |
| completeness | unimplemented functionality, `todo!()` gaps, future-feature constraints |
| metering | gas charging and recursion/size/cache-DoS bounds |
| perf | speed-only optimizations |
| cleanup | refactor, rename, error-type unification, design questions, docs |
| testing | test infra and missing tests |

No roadmap `TODO*.md` files — items live inline at the owning code site. To
verify (prints nothing if clean):

```bash
grep -rn "TODO" --include='*.rs' third_party/move/mono-move \
  | grep -v 'todo!\|unimplemented!\|unreachable!' \
  | grep -vE 'TODO\(((correctness|security|completeness|metering|perf|cleanup|testing)(, )?)+\):'
```
