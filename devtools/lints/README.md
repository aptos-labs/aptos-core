# aptos_lints

Custom lints for aptos-core, built on [dylint](https://github.com/trailofbits/dylint).
A dylint lint is a `LateLintPass` compiled against `rustc_private`, so it sees
fully resolved types, HIR and MIR — everything clippy can see.

## Lints

| lint | what it flags | applies to (see `/dylint.toml`) |
|---|---|---|
| `no_std_hash_map` | `std::collections::{HashMap, HashSet}`, `hashbrown`, and aliases such as `FxHashMap`; use `shared_dsa::{UnorderedMap, UnorderedSet}` (no iteration exposed) or `BTreeMap`/`BTreeSet` | consensus-critical mono-move crates |

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
the lint:

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

## Adding a lint

1. New module in `src/`, `declare_lint!` + `impl_lint_pass!`, a `Scope` field
   read in `register_lints` (`src/lib.rs`), a `Scope` entry in
   `scope::Config`.
2. `ui/<lint>.rs` with the cases it must and must not flag; run
   `cargo test`, review `/tmp/<lint>.stage-id.stderr`, copy it to
   `ui/<lint>.stderr`.
3. A scope entry in `/dylint.toml`.

## Maintenance

The crate pins its own nightly and a matching `clippy_utils` revision.
`cargo dylint upgrade devtools/lints` bumps both together; rustc's internal
APIs change between nightlies, so expect small fixes on each bump.
