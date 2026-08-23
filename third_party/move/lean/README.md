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

| Package | Library | Purpose |
|---|---|---|
| `move` | [`Move`](move/Move/README.md) | The Leaner Move source language: surface, source contracts, the `verify` proof engine, and lowering to XIR, with its regressions under `Move/Tests`.  **Start here.** |
| `move-model` | [`MoveModel`](move-model/MoveModel/README.md) | A logical model of Move bytecode: stackless IR, execution semantics, prover stages, and masm/Move source embedding, with its regressions under `MoveModel/Tests`.  What `Move` compiles into, and usable on its own. |

Two Lake packages, each holding the library of the same name; `move` depends on
`move-model`.  A downstream project requires whichever it needs:

```toml
[[require]]
name = "move"
path = "<checkout>/third_party/move/lean/move"
```

The language is defined in [`leaner-move.md`](move/Move/leaner-move.md), the
verification design in
[`verification-design.md`](move/Move/verification-design.md), and what the
surface does *not* yet handle in
[`project-plan.md`](move/Move/project-plan.md).  Each library README owns its
architecture, module index, and roadmap.

## Build and test

Install the repository-pinned Lean toolchain with the standard development
setup script. The script verifies the downloaded release archive and adds its
tools to the shell profile.

```bash
scripts/dev_setup.sh -p -l
source "$HOME/.profile"
```

```bash
cd move-model && lake build     # the logical model
cd move       && lake build     # Leaner Move (builds move-model first)
```

The core libraries do not require the Aptos CLI. The regression suites do
require the exchange frontend. Build its lightweight single-file entrypoint and
set `APTOS_MOVE_EXCHANGE` in your shell profile; this is **highly recommended**
for normal Leaner development because it avoids rebuilding the full Aptos CLI
and greatly improves edit/test turnaround. Then run each package's suite:

```bash
cargo build -p aptos-move-cli --bin aptos-move-exchange
export APTOS_MOVE_EXCHANGE="$PWD/../../../target/debug/aptos-move-exchange"
cd move-model && lake test
cd move       && lake test
```

`APTOS_CLI=<path-to-aptos>` remains supported for the full `aptos move
exchange` command.

Proof cost is tracked with `scripts/bench-proofs.sh`; the encoding's cost
analysis is in
[`performance-analysis.md`](move/Move/performance-analysis.md).
