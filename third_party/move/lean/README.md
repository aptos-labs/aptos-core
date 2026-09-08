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
| `transpiler` | [`Transpiler`](transpiler/transpile-design.md) | The Move-to-Leaner transpiler: runs `aptos move exchange --format ast` on Move sources and prints Leaner Move (`lake exe transpile`), with baselines (`Tests/Programs/<name>.move` beside its generated `<Name>.lean`) and an elaboration gate.  Requires `move` for its tests. |
| `leaner-rust` | [`LeanerRust`](designs/rust-mir-design.md) | The initial Rust semantic profile plus the project-owned Rustc Public exporter spike. It imports into the shared `leaner-ir` boundary and stops rustc before code generation. |
| `leaner-e2e-tests` | [end-to-end baselines](leaner-e2e-tests/README.md) | Discoverable Move→LeanerLang and Rust→LeanerLang source/result baselines. It temporarily depends on `transpiler` while the compatibility adapters are migrated. |

Each production Lake package holds the library of the same name;
`leaner-e2e-tests` is deliberately test-only and may depend on all layers it
exercises. `move` depends on `move-model`, while `leaner-rust` depends on
`leaner-ir`. A downstream project requires whichever it needs:

```toml
[[require]]
name = "move"
path = "<checkout>/third_party/move/lean/move"
```

The current cross-package design documents live in [`designs/`](designs/);
executed or superseded ones are kept under
[`designs/historical/`](designs/historical/). The
profile-general Leaner source language is being designed in
[`designs/leaner-lang.md`](designs/leaner-lang.md). Its implemented Move profile is
defined in [`leaner-move.md`](move/Move/leaner-move.md), the verification
architecture in [`verification-design.md`](move/Move/verification-design.md),
and current surface coverage and gaps in
[`project-plan.md`](move/Move/project-plan.md).  Each library README owns its
architecture, module index, and roadmap.

The `leaner-rust` package uses a project-owned Rustc Public exporter to
import borrow-checked MIR into validated Leaner IR, then reproduce readable
Rust source from that IR. Charon remains a differential-testing aid, and
raw-pointer support is staged. The full decision is described in
[`designs/rust-mir-design.md`](designs/rust-mir-design.md).

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
cd transpiler && lake build     # the Move-to-Leaner transpiler and `transpile`
```

The core libraries do not require the Aptos CLI. The regression suites do
require the exchange frontend. Build the standalone Move CLI and set
`APTOS_MOVE_CLI` in your shell profile; this is **highly recommended** for
normal Leaner development because it avoids the full Aptos CLI and greatly
improves edit/test turnaround. Then run each package's suite:

```bash
cargo build -p aptos-move-cli --features binary --bin move
export APTOS_MOVE_CLI="$PWD/../../../target/debug/move"
cd move-model && lake test
cd move       && lake test
cd transpiler && lake test
```

`APTOS_CLI=<path-to-aptos>` remains supported for the full `aptos move
exchange` command. The standalone Move CLI receives `exchange` directly, so
its equivalent command is `move exchange`.

Proof cost is tracked with `scripts/bench-proofs.sh`; the encoding's cost
analysis is in
[`performance-analysis.md`](move/Move/performance-analysis.md).
