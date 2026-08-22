# *WIP* Leaner Move

This experimental project is a source language for writing Move contracts in
Lean 4, together with a verifier that proves contracts directly against the
authored source and a compiler that lowers the same source to production Move
bytecode.

A module is ordinary Lean syntax.  `spec` attaches a contract declaratively and
`verify` turns it into a theorem checked by the Lean kernel:

```lean
import Move

open scoped Move Move.Spec

module Account where

  struct Balance has Key where
    value : U64

  entry fun deposit (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Balance[addr].value
    value := *value + amount

  spec deposit (addr : Address) (amount : U64) where
    requires existsAt<Balance>(addr);
    modifies Balance[addr];
    ensures Balance[addr].value = old(Balance[addr].value) + amount;
    aborts_if ¬old(Balance[addr].value).toNat + amount.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify deposit
```

The same source has two distinct uses:

1. **Source verification.**  `verify f` produces `f.verified : f.contract`,
   proved over the generated relational semantics of the authored function.
2. **Executable compilation.**  Selected declarations are lowered through typed
   base LCNF, `Move.Compiler.LIR`, and `MoveModel.IR` into versioned XIR, then
   compiled by the complete compiler-v2 pipeline and checked by the production
   Move bytecode verifier.

A compiler-correctness theorem connecting `f.verified` to the emitted bytecode
remains future work; the prototype does not conflate those claims.  XIR is a
compiler exchange format, not a proof artifact.

## Components

| Package | Purpose |
|---|---|
| [Move](Move/README.md) | The Leaner Move source language: surface, source contracts, the `verify` proof engine, and lowering to XIR.  **Start here.** |
| [MoveModel](MoveModel/README.md) | A logical model of Move bytecode: stackless IR, execution semantics, prover stages, and masm/Move source embedding.  The target the compiler lowers into, and usable on its own. |
| [`Tests`](Tests) | Source-verification, interpreter, compiler-lowering, and reference-elimination regressions, including cross-module Lean-authored programs. |

The language is defined in [`leaner-move.md`](Move/leaner-move.md), the
verification design in
[`verification-design.md`](Move/verification-design.md), and what the surface
does *not* yet handle in [`project-plan.md`](Move/project-plan.md).  Each
package README owns its architecture, module index, and roadmap.

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

The core library does not require the Aptos CLI.  Compilation-backed and
frontend-backed examples and tests do: build `aptos` (the checkout-local debug
binary is selected automatically), or select another binary with `APTOS_CLI`,
then run:

```bash
APTOS_CLI=<path-to-aptos> lake build Examples
APTOS_CLI=<path-to-aptos> lake test
```

`lake test` limits the number of concurrent Lean test compilations to avoid
exhausting file descriptors when many metaprogramming-heavy test modules need
to be rebuilt together.

Proof cost is tracked with `scripts/bench-proofs.sh`; the encoding's cost
analysis is in [`performance-analysis.md`](Move/performance-analysis.md).
