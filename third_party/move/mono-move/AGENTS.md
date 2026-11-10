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
  (which expose no ordered iteration, so nothing can depend on hash order by
  accident) or `BTreeMap`/`BTreeSet` when ordered iteration is needed. The
  lint only removes accidental dependence on hash order; it does not make a
  site correct. See "Choosing a map" below.
- `raw_memory_access` — no raw pointers, raw-pointer method calls, or
  `ptr::`/`slice::from_raw_parts` calls outside `mem` and `alloc`; see
  "Raw memory access".
- `unsafe_cell_reborrow` — the discipline for `&self` methods that mutate
  through `UnsafeCell` fields (the native context, the root pool): one reborrow per
  cell per method, held for the whole method; no call that may reborrow a
  held cell; no escape; nothing that can reach the struct handed to unseen
  code while holding. `VMValue` impls must not reach a `NativeContext`. The
  rules are on `ProductionNativeContext` ("Cell discipline") and in
  `devtools/lints/README.md`.

### Choosing a map

Ask two questions at every site:

1. **Does anything walk the map?** If not, use `UnorderedMap`. If so, use a
   `BTreeMap`, but only when the key's `Ord` is the same on every build (no
   pointers, interned ids or `TypeId`). Otherwise use
   `UnorderedMap::into_iter_unordered` and make the result order-independent
   explicitly (sort, or collect into an ordered structure).
2. **Can an attacker choose the keys?** (addresses, struct tags, identifiers,
   anything from a transaction.) If so, the map must be collision resistant.
   The only options are a `BTreeMap` or a hash map with a randomly keyed
   hasher: `UnorderedMap<K, V, ahash::RandomState>`. The default
   `UnorderedMap` hasher (Fx) is **not** collision resistant; use it only for
   keys an attacker cannot choose, or that are already cryptographic hashes
   (e.g. `StateKey`).

A randomly keyed hasher is safe for determinism only because `UnorderedMap`
exposes no ordered iteration. Say which case applies in a comment on the field.

Where a plain hash map really is the better structure, suppress the lint with
a comment saying why; that is an intended outcome, not a workaround.

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
