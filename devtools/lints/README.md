# aptos_lints

Custom lints for aptos-core, built on [dylint](https://github.com/trailofbits/dylint).
A dylint lint is a `LateLintPass` compiled against `rustc_private`, so it sees
fully resolved types, HIR and MIR — everything clippy can see.

## Lints

| lint | what it flags | applies to (see `/dylint.toml`) |
|---|---|---|
| `no_std_hash_map` | `std::collections::{HashMap, HashSet}`, `hashbrown`, and aliases such as `FxHashMap`; use `shared_dsa::{UnorderedMap, UnorderedSet}` (no ordered iteration exposed) or `BTreeMap`/`BTreeSet` | consensus-critical mono-move crates |

### What `no_std_hash_map` does not guarantee

The lint removes one class of bug: code that depends on hash order by
accident. It does not make a site deterministic or safe on its own.

- A `BTreeMap` iterates in the order of its key's `Ord`. That is only
  deterministic if `Ord` is the same on every build: keys ordered by a
  pointer, an interned id or a `TypeId` are not.
- A walk whose order can reach an output still needs explicit handling (sort,
  or collect into an ordered structure). `UnorderedMap::into_iter_unordered`
  is the one way to walk an `UnorderedMap`, named so the obligation is visible.
- `UnorderedMap`'s default hasher (Fx) is not collision resistant. A map keyed
  by attacker-chosen data needs a `BTreeMap` or
  `UnorderedMap<K, V, ahash::RandomState>`.
- Where a plain hash map is the better structure, a commented `allow` (see
  "Suppressing a hit") is an intended outcome.

`third_party/move/mono-move/AGENTS.md` ("Choosing a map") has the guideline
for picking a map at a given site.

## Running

```bash
scripts/dev_setup.sh                               # installs cargo-dylint + dylint-link
third_party/move/mono-move/scripts/dylint.sh       # all mono-move crates
cargo dylint --all -p mono-move-core               # one crate
(cd devtools/lints && cargo test)                  # the lints' own UI tests
```

The first run builds the lint library and a driver for its pinned nightly
(`rust-toolchain.toml`); both are cached afterwards. CI runs the same script
(`.github/workflows/mono-move-lints.yaml`) on mono-move PRs only.

## Scoping

Lints are scoped by **crate name** in `/dylint.toml`, not by path or inside
the lint. The file lives at the repo root because dylint reads it from the
workspace being linted, the same way clippy finds `clippy.toml`:

```toml
[aptos_lints.no_std_hash_map]
crates = ["mono-move-*"]           # trailing `*` glob
exempt = ["mono-move-testsuite"]   # subtracted
```

A lint with no `crates` entry applies everywhere. Promoting a lint from one
subsystem to the whole repo is a config change.

## Suppressing a hit

On the item or statement:

```rust
#[cfg_attr(dylint_lib = "aptos_lints", allow(no_std_hash_map))]
```

(`dylint_lib` is a cfg dylint sets, so the attribute is inert under a normal
build and never triggers `unknown_lints`.) Say why in a comment.

## Layout

```
src/lib.rs            registers every lint
src/scope.rs          per-lint crate scoping, read from /dylint.toml
src/<lint>.rs         one module per lint
ui/<lint>.rs          UI test: cases the lint must and must not flag
ui/<lint>.stderr      the exact diagnostics that file must produce
```

`ui/` is rustc's and clippy's convention for lint tests ("UI" as in what the
user sees): a source file is compiled through the driver and its diagnostics
are diffed, byte for byte, against the blessed `.stderr`. They pin a lint's
observable behaviour, so a change to what a lint flags or says shows up as a
`.stderr` diff in review.

## Adding a lint

1. New module in `src/`, `declare_lint!` + `impl_lint_pass!`, a `Scope` field
   read in `register_lints` (`src/lib.rs`), a `Scope` entry in
   `scope::Config`.
2. `ui/<lint>.rs` (with the repo's license header, like any `.rs` file)
   with the cases it must and must not flag; run `cargo test`, review
   `/tmp/<lint>.stage-id.stderr`, copy it to `ui/<lint>.stderr`.
3. A scope entry in `/dylint.toml`.

## Maintenance

The crate pins its own nightly and a matching `clippy_utils` revision.
`cargo dylint upgrade devtools/lints` bumps both together; rustc's internal
APIs change between nightlies, so expect small fixes on each bump.
