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
  (which expose no iteration, so nothing can depend on hash order) or
  `BTreeMap`/`BTreeSet` when ordered iteration is needed.

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
