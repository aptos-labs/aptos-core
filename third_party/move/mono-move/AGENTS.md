# mono-move

Conventions for every crate under `third_party/move/mono-move/`. Crate-specific
docs live in each crate's `AGENTS.md`; design rationale in `docs/`.

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
