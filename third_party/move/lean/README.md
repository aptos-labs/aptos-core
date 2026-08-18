# *WIP* Lean formalization of Move

This experimental project formalizes a substantial but deliberately limited
fragment of Move stackless bytecode, its execution semantics, and selected
stages of the Move Prover pipeline in Lean 4.  It can also execute and verify
examples embedded as masm or Move source.  It is not a formalization of the
full Move language or the complete production prover; notably, closures,
generics, and several other language features are outside the modeled
fragment.

## Components

| Package | Purpose |
|---|---|
| [Move IR](Move/IR/README.md) | Stackless IR syntax and specifications, relational semantics, interpreter, typing and checking certificates, reusable proof infrastructure, and reference elimination. |
| [Move Prover](Move/Prover/README.md) | State-polymorphic IVL, weakest preconditions, loop cutting, IR-to-IVL compilation, simulation, and end-to-end adequacy. |
| [Move frontend](Move/Frontend/README.md) | Embedded masm and Move-source elaborators backed by `aptos move exchange`, including optional reference elimination. |
| [`Move/Examples`](Move/Examples.lean) | Hand-written and frontend-backed execution and verification examples. |
| [`Tests`](Tests) | Interpreter and reference-elimination regression tests. |

Each package README owns its architecture, module index, completeness status,
and roadmap.  Reference elimination is proved at the conceptual IR-model
level under explicit frontend checking certificates; its precise boundary and
remaining certificate-refinement work are documented in the
[IR roadmap](Move/IR/README.md#completeness-and-roadmap).  The correctness
developments for the modeled fragment contain no `sorry`.  This is a statement
about these Lean definitions and hypotheses, not the full Move language or the
production reference-elimination pass.

## Small example

```lean
def prog : Program := masm% "
module 0x42::count_down

fun count_down(x: u64): u64
    ensures result == 0
l1: copy_loc x
    ...
"

#eval interpFun prog 100 0 [] [.u64 5]
theorem verified : Verified prog 0 := by ...
```

The corresponding `move%` form accepts a self-contained Move module with
compiler-v2 `spec` blocks.  Reference-bearing programs can use `masmElim%` or
`moveElim%`; see the [frontend guide](Move/Frontend/README.md).

## Build and test

Install the repository-pinned Lean toolchain with the standard development
setup script. The script verifies the downloaded release archive and adds its
tools to the shell profile.

```bash
scripts/dev_setup.sh -p -l
source "$HOME/.profile"
```

```bash
lake build
```

The core library does not require the Aptos CLI.  Frontend-backed examples and
tests do: build `aptos`, place it on `PATH` or set `APTOS_CLI`, then run:

```bash
APTOS_CLI=<path-to-aptos> lake build Examples
APTOS_CLI=<path-to-aptos> lake test
```

The build contains no admitted reference-elimination theorem.  Setup,
supported source syntax, test conventions, and known frontend limitations are
documented in the [frontend README](Move/Frontend/README.md).
