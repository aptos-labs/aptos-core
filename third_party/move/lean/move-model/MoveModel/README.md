# MoveModel: a logical model of Move bytecode

This directory formalizes a substantial but deliberately limited fragment of
Move stackless bytecode, its execution semantics, and selected stages of the
Move Prover pipeline in Lean 4.  It is the target model that
[Leaner Move](../../move/Move/README.md) compiles into, and it stands on its own as a
logical account of Move bytecode: programs can be embedded from masm or Move
source, executed, and verified without authoring anything in Leaner.

It is not a formalization of the full Move language or the complete production
prover; notably, closures, recursive data, and several other language features
remain outside the modeled fragment.  True Move generics are preserved by
executable compilation and use a separate finite monomorphized view for IR
verification.

## Components

| Package | Purpose |
|---|---|
| [IR](IR/README.md) | Stackless IR syntax and specifications, relational semantics, interpreter, typing and checking certificates, reusable proof infrastructure, and reference elimination. |
| [Prover](Prover/README.md) | State-polymorphic IVL, weakest preconditions, loop cutting, IR-to-IVL compilation, simulation, and end-to-end adequacy. |
| [Frontend](Frontend/README.md) | Embedded masm and Move-source elaborators backed by `aptos move exchange`, including optional reference elimination. |
| [`Tests`](Tests.lean) | Interpreter, IR, frontend, and prover regressions, including hand-written and frontend-backed verification examples under [`Tests/Prover`](Tests/Prover/). |

Each package README owns its architecture, module index, completeness status,
and roadmap.

## Embedding masm and Move source

A program can be written directly as masm and elaborated into a first-order IR
term, then executed and verified in the same file:

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
`moveElim%`; see the [frontend guide](Frontend/README.md).

Elaboration shells out to `aptos move exchange`, so these forms need the Aptos
CLI even though the core library does not — see *Build and test* in the
[tree README](../../README.md).

## Status

Reference elimination is proved at the conceptual IR-model level under explicit
frontend checking certificates; its precise boundary and remaining
certificate-refinement work are documented in the
[IR roadmap](IR/README.md#completeness-and-roadmap).  The correctness
developments for the modeled fragment contain no `sorry`, and the build
contains no admitted reference-elimination theorem.  This is a statement about
these Lean definitions and hypotheses, not about the full Move language or the
production reference-elimination pass.
