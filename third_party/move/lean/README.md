# *WIP* Lean Formalization of Move

> This is an experimental, AI generated model of Move stackless bytecode, its execution semantics, and 
> specification. This is intended in the first place as a basis for describing and verifying the correctness
> of essential steps in the Move Prover's verification pipeline. But it may also be used to specify and verify 
> Move programs in Lean, where as input either `.masm` or `.move` files are supported.

A [Lean 4](https://lean-lang.org) formalization of the core of the Move language toolchain,
the so-called stackless bytecode in [IR](./Move/IR). 

Supports translation into the [IVL](./Move/Prover/Ivl) which mimics the Move prover with all its individual steps (WIP).

## Prover Support

The formalization models the Move Prover's pipeline — specification
injection into a Boogie-style IVL and a weakest-precondition calculus
defined in Lean — with proven soundness of the WP calculus and of the
loop-to-DAG reduction, and the end-to-end adequacy theorems stated.  The
architecture, the theorem map (paper concept → Lean artifact → status),
the `sorry` inventory, and the semantic fine print are documented in
[design/prover.md](design/prover.md).

## The masm and Move-source frontends

Examples can be authored as **masm strings embedded in Lean** (the textual
Move assembler syntax of `tools/move-asm`, extended with specification
clauses):

```lean
def prog : Program := masm% "
module 0x42::count_down

fun count_down(x: u64): u64
    ensures result == 0
l1: copy_loc x
    ...
"

#eval interpFun prog 100 0 [] [.u64 5]          -- evaluate
theorem verified : Verified prog 0 := by ...     -- verify
```

At elaboration time, `masm%` (`Move/Frontend/Elab.lean`) runs the Aptos CLI's
`exchange` command (implemented in `aptos-move/cli/src/exchange/`),
which parses and assembles the source
with the real assembler, verifies the module, lifts it with the real
`StacklessBytecodeGenerator`, and dumps the basic-block CFG — plus the
translated spec clauses and the loop metadata (natural loops, written
locals/resources) — as JSON; the Lean side decodes and splices it as a
first-order term (`MProgram.toProgram …`), so proofs unfold it with `simp`
like any hand-written program.  The JSON schema is defined normatively by
the serde-annotated Rust datatypes of the `move-model-exchange` crate
(`third_party/move/move-model/exchange`).  Assembler/frontend errors
surface as positioned elaboration errors.

- The elaborators need the Aptos CLI: set `APTOS_CLI` to the binary, or
  have `aptos` on `PATH`.  The core library builds without it; the
  embedding examples live in the separate lake target `Examples`
  (`lake build Examples`).  Dumps are cached in the temp directory (keyed
  by source content and CLI binary), so unchanged embedded sources do not
  re-run the CLI on re-elaboration.
- The spec clauses (`requires`/`aborts_if`/`ensures`/`modifies` after the
  function header, `invariant <label>: <exp>` for loop headers) are part of
  move-asm's grammar (parse-only; the assembler ignores them).  Spec
  expressions support the usual operators, `old(..)`,
  `global<T>(..)`/`exists<T>(..)`, field selection, `result`, and typed
  quantifiers `forall`/`exists x: u64|address . e` — the binder's declared
  domain type bounds the range (there are no explicit type-test
  predicates; well-formedness is derived from declared types throughout).
- Supported masm fragment: u64/bool/address/struct values, arithmetic,
  comparisons, logical operators, `pack`/`unpack`, resource instructions,
  direct calls, labels and branches.  References assemble, elaborate, and
  *run* (the interpreter executes them); verifying borrow-based code goes
  through the in-model reference elimination (`IR.refElimFun`, see
  `Examples.BorrowAccount`) — the frontend does not yet apply it
  automatically — the `masmElim%`/`moveElim%` elaborators do: they run
  the whole-program elimination at elaboration time, so borrow-based
  source verifies directly (see `Examples.ElimSource`).  Vectors exist
  in the core model (values, the `vec_*`
  operations, element borrows, spec-level `len`/`index` — see
  `Tests/Interp/Vectors.lean`) but the frontends do not translate the
  vector natives yet.  Generics, enums, vectors in source, other integer
  widths, and casts are rejected with clear errors.
- Evaluation uses the computable, fuel-based interpreter
  (`Move/IR/Interp.lean`); its agreement theorem with the relational
  semantics is a roadmap item.

**Move source** works the same way through `move%`: a self-contained Move
module (no dependencies) is compiled with compiler v2, so specifications
come from the genuine `spec` blocks — including inline
`spec {{ invariant …; }}` loop invariants.  Typing assumptions are not
part of the specification: the verification pipeline derives them from
the declared types (`IsValid`), as the real prover injects `WellFormed`:

```lean
def prog : Program := move% "
module 0x42::account {
    struct Account has key { balance: u64 }
    fun take(addr: address): u64 acquires Account { ... }
    spec take {
        aborts_if !exists<Account>(addr);
        ensures result == old(global<Account>(addr).balance);
        modifies global<Account>(addr);
    }
}
"
```

See `Move/Examples/MoveSource.lean` for verified examples authored this
way.  The same command exports at package level:
`aptos move exchange --package-dir <pkg>` writes one
`<module>.exchange.json` per target module (modules outside the supported
fragment are skipped with a warning); `--masm-file`/`--move-file` with
`--out-file` are the single-file modes the elaborators use.

## Building

Install [elan](https://github.com/leanprover/elan) (the Lean toolchain
manager); the toolchain version is pinned by `lean-toolchain`:

```bash
curl -sSf https://elan.lean-lang.org/elan-init.sh | sh -s -- -y
cd third_party/move/lean
lake build
```

The build must succeed with a warning only for the one inventoried sorry
(`refElim_correct`).

Building the `Examples` target additionally requires the Aptos CLI,
whose `exchange` command is invoked by the `masm%`/`move%` elaborators:
build it with `cargo build -p aptos` and make sure `aptos` is on `PATH`
(or set `APTOS_CLI` to the binary):

```bash
lake build Examples
```

## Testing

The interpreter has a unit-test suite under `Tests/`, with test programs
authored as embedded Move source (`move%`/`moveM%`).  A test case is a
`#test` command (`Tests/Common.lean`) — a `#guard` that reports what
actually happened: `#test e = v` compares against an expected outcome,
written with the helper constructors (`okU64 1`, `okBool true`, `okUnit`,
`okVals`, `okRet mem vs`, `aborted code`, `abortedIn mem code`), and
`#test e matches pat` checks a pattern (for wildcard expectations).  A
failure reads like a unit-test assertion:

```
test failed
  expected: .ok (.ret [] [.u64 1])
  actual:   .ok (.ret [] [.u64 0])
```

Tests are evaluated at elaboration time and fail the build, so running the
tests is building the test target — `Tests` is the configured `lake test`
driver (it invokes the Aptos CLI, like the examples):

```bash
APTOS_CLI=<path-to-aptos> lake test
```

The build replays the (expected) `sorry` warnings of the upstream theory
modules; to see only failures, add `--log-level=error`.

Covered: arithmetic and its abort semantics, comparisons and the
`>`/`>=`/`!=` normalizations, control flow, recursion, user abort codes,
fuel exhaustion, structs, the reference semantics (local, field, nested,
frozen, and global borrows), the resource instructions, and — for the
borrow-based modules — agreement of `refElimFun`'s output with the
original program on the same runs (executable evidence for
`refElim_correct`).

Function and resource ids in a dumped module are positional; tests resolve
them by name via `MProgram.funId`/`resourceId` on the first-order
representation (`moveM%` retains the names; `move%` is its composition
with `MProgram.toProgram`).

## Roadmap

1. ~~Bytecode CFG + deep spec expressions + block-graph WP with loop
   invariant rule, loop cutting, spec injection, examples~~ (done)
2. ~~Prove the simulation and adequacy theorems~~ (done: one master
   induction over the execution derivation proves simulation, contract
   conformance and type preservation at once, under the source-typing
   discipline `WfProg` — see [design/prover.md](design/prover.md))
3. ~~Type-derived well-formedness: define `IsValid` over the declared
   types and assume it at entry, for memory, after loop havoc, and at
   quantifiers (typed domains) — replacing the synthesized and
   hand-written well-formedness assumptions~~ (done)
4. Reference elimination on the bytecode level (TACAS'22 §3.1) —
   ~~reference semantics and the elimination pass, with a verified
   borrow-based example~~ (done); ~~vector core operations, so the
   elimination can be designed once against dynamic element indices~~
   (done); ~~checkout call semantics, giving references crossing call
   boundaries their meaning (MVP's `Mut` threading in the source
   semantics)~~ (done); ~~the full-semantics intra-procedural pass:
   mutation values with dynamic `isParent` write-back dispatch, borrow
   graph + liveness dataflow, cross-block borrows, block/edge splitting,
   loop-target extension~~ (done); ~~cross-call references with borrow
   summaries (`refElimProg`), including a caller verified through its
   callee's contract~~ (done); ~~run the pass in the frontend pipeline
   (`masmElim%`/`moveElim%`), with borrow-based Move source verified
   directly~~ (done); remaining: prove `refElim_correct`
5. Interpreter/semantics agreement theorem (`interpFun` vs `RunFrom`)
6. Data invariants and global (inductive/update) invariants, invariant
   suspension, access/modification analysis (TACAS'22 §3.2)
7. Higher-order function values: closures, parameter/field variants,
   behavioral predicates, dynamic invocation (FMCAD'26 §III)
8. State labels: labeled one-/two-state predicates, existential intermediate
   states, abort synthesis over label DAGs (FMCAD'26 §III-E)
9. Generics and monomorphization, including type-unification-based instance
   discovery (TACAS'22 §3.3)
10. Formalized WP-based specification inference over the bytecode CFG
   (FMCAD'26 §IV)
