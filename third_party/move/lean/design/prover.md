# Prover Support

How the Lean formalization models the Move Prover's verification pipeline:
the correspondence between the prover's components and the Lean artifacts,
the proven theorems, and the semantic fine print.

Canonical references:

- **[TACAS'22]** Dill, Grieskamp, Park, Qadeer, Xu, Zhong: *Fast and Reliable
  Formal Verification of Smart Contracts with the Move Prover*
  (`../../doc/paper21`, arXiv:2110.08362).
- **[FMCAD'26]** Grieskamp, Zhang, Kashyap, Silverman: *Formal Verification of
  Imperative First-Class Functions in Move*.

## Architecture

The input mirrors the prover's intermediate representation — **stackless
bytecode as a control-flow graph of basic blocks** — and the output mirrors
Boogie's — **labeled basic blocks with gotos**.  Instead of formalizing
Boogie's VC generation, the verification-condition generator is a
**weakest-precondition calculus defined in Lean**, which also prepares the
ground for formalizing MVP's WP-based *specification inference*:

```
IR         bytecode CFG: three-address instrs over locals, block     Move/IR/
           terminators (jump/branch/ret/abort), loop-header
           annotations, contracts in a deep spec-expression
           syntax (SpecExp, mirroring move-model ExpData)
    |
    |  compileFun  (specification injection, TACAS'22 Fig. 8/9;    Move/Prover/Translate/
    |               SpecExp denoted into shallow predicates
    |               over the verification state VState)
    v
IVL        Boogie-style: labeled blocks, guarded goto, ret;         Move/Prover/Ivl/
           assign/havoc/assume/assert; loop-header annotations
           (invariant, target relation, member set)
    |
    |  wpB   (fuel-based WP over the block graph; invariant rule    Move/Prover/Ivl/Wp.lean
    v         at annotated headers, back-edge cut)
Lean Prop
```

Key design decisions:

- **Code is three-address, specs are expressions** — as in the real IR: all
  instruction operands are locals; contracts and loop invariants carry
  the deep syntax `SpecExp` (locals, constants, operators,
  `global`/`exists` with optional **memory label**, quantifiers with de
  Bruijn binders carrying their declared domain type).  `old(..)` does not exist at this
  level: it is represented by accesses against the `preLabel` snapshot, the
  `SaveMem` mechanism of the real pipeline.  Spec arithmetic is over
  unbounded integers (`num`), with the relational semantics `EvalSpec`
  (quantifiers preclude an executable evaluator).
- **The IVL is state-polymorphic with shallow conditions**: block structure
  is deep, guards/assertions are Lean predicates `σ → Prop`, updates are
  functions, havoc is a relation.  The guarded `goto` fuses Boogie's
  nondeterministic goto with the head `assume`s of its `if-goto` desugaring.
- **Loops are annotations, and the WP rule is the invariant rule**: `wpB`
  recurses along a rank order (block ids follow code layout, as bytecode
  offsets do); a rank-non-increasing edge is a back edge and contributes
  only the target header's invariant — the back-edge cut of MVP's
  `LoopAnalysisProcessor`; an annotated header contributes the invariant
  rule (assert `I`; havoc targets; assume `I`).  The corresponding *program
  transformation* `loopCut` (header havoc/assume, back edges retargeted to
  fresh `assert I; stop` blocks) is proven WP-equivalent (`loopCut_wp`) and
  acyclic (`loopCut_acyclic`) — `wpB` computes exactly the VC of the DAG the
  real pipeline sends to Boogie.
- **Move aborts become data flow**: the verification state `VState` carries
  the abort flag (`$abort_flag`/`$abort_code` of the real Boogie encoding);
  every compiled instruction is guarded on it, the compiled terminator
  routes to the abort exit block when it is set, and the two exit blocks
  carry the TACAS'22 Fig. 8 exit assertions, including the **biconditional**
  reading of `aborts_if`.
- **Calls are real in the source semantics** (the callee body executes); the
  *compilation* of a call is the opaque contract schema `assert requires;
  havoc (abort branch ∨ ensures ∧ frame)`.  Modular verification appears as
  hypotheses of the meta-theorems.
- **Source blocks map 1-1 to IVL blocks** (source `b` ↦ label `b + 1`, plus
  an entry stub and two exit blocks), so the simulation statements align
  block-wise.

## Theorem map

| Paper / design concept | Lean artifact | Status |
|---|---|---|
| WP soundness over the block graph, incl. the loop invariant rule | `Prover.Ivl.wpB_sound`, `wpB_safe`, `wpB_post` (`Prover/Ivl/WpSound.lean`) | **proven** |
| WP exactness on acyclic programs | `Prover.Ivl.wpB_complete` (`Prover/Ivl/LoopCut.lean`) | **proven** |
| Loop-to-DAG reduction (`LoopAnalysisProcessor`, TACAS'22 §3) | `Prover.Ivl.loopCut`, `loopCut_wp`, `loopCut_acyclic` (`Prover/Ivl/LoopCut.lean`) | **proven** |
| Completeness of loop-target analysis / fat-loop structure | side conditions `Prover.Ivl.WfProgram` (`Prover/Ivl/Wp.lean`) | definition |
| Stackless bytecode CFG (`stackless_bytecode.rs`) | `IR.Instr`, `Term`, `Block`, `Cfg` (`IR/Syntax.lean`) | definition |
| Spec expressions (`ExpData`), `SaveMem`/memory labels | `IR.SpecExp`, `EvalSpec`, `Holds` (`IR/Spec.lean`) | definition + `evalSpec_det`, inversion pack **proven** |
| Reference semantics (references as runtime values) | `IR.RefTarget`, the `RunFrom` borrow/read/write rules, `MoveState.readTarget`/`writeTarget` (`Value.lean`, `Semantics.lean`) | definition |
| Vectors (values, `vec_*` operations, element borrows, spec `len`/`index`) | `IR.Value.vector`, `Ty.vector`, the `Oper.vec*` `sem` cases, `RunFrom.borrowVecElem*`, `SpecExp.len`/`SpecBinop.index`; preservation cases in `WfOp.sem_preserves` | definition + preservation **proven** |
| Cross-call references (MVP's `Mut` threading, `call r := f(r)`) | checkout call rules `RunFrom.callRefOk/callRefAbort` with `Cfg.extendRets`, `checkoutLocals`, `MoveState.writeTargets`, `reRootRet` (`Semantics.lean`); excluded at well-typed call sites by `IsValid.not_ref`/`IsValidList.locRefTargets_eq_nil` | definition + typed-exclusion **proven** |
| Reference elimination (TACAS'22 §3.1, `eliminate_imm_refs` + `borrow_analysis` + `memory_instrumentation`) | `IR.refElimFun` (`RefElim.lean`): imm-ref pre-pass, liveness + borrow-graph dataflow (union join), mutation checkout/derivation codegen, death-point write-backs with `is_parent` dispatch via block/edge splitting, loop-target extension | definition, executable |
| Inter-procedural borrow summaries (`summarize`/`construct_hyper_edges`, hyper edges) | `IR.FunSummary`, `IR.summarize`, `IR.refElimProg` (Kleene iteration); callers instantiate summarized derivations as hyper-path edges, `&mut` params become value-in/finals-out (`call r := f(r)`, the ret extension) | definition, executable |
| Spec-level mutation dereference (Boogie's `$Dereference`) | `IR.SpecExp.mutVal` (`Spec.lean`); `IsValid` at `&mut` requires the mutation shape, so `mutVal` on parameters is entry-assumed well-defined | definition + inversion **proven** |
| Correctness of reference elimination | `IR.refElim_correct` (`RefElim.lean`): forward simulation up to `AgreeOutcome` (abort memory may lack deferred write-backs) | `sorry` |
| Type-indexed global memory (TACAS'22 §2) | `IR.Memory`, `MoveState` (`IR/State.lean`) | definition |
| Multisorted `WellFormed`/`$IsValid` injection (Boogie prelude) | `IR.IsValid` (`IR/Typing.lean`), `TypedArgs`/`TypedMemory` (`IR/Syntax.lean`); `Prover.Translate.typedEntry`, typed havoc in `denoteLoopSpec` | definition + inversion pack **proven** |
| Contracts, biconditional `aborts_if` (FMCAD'26 §II) | `IR.Contract`, `SatisfiesContract` (`IR/Contract.lean`, `Semantics.lean`) | definition |
| Specification injection (TACAS'22 Fig. 8/9, App. A) | `Prover.Translate.compileInstr`, `compileBlock`, `compileFun` (`Prover/Translate/Compile.lean`) | definition |
| Frame of `modifies` havoc | `Prover.Translate.callRel` normal branch; `writeLocals_memory` (`Prover/Translate/Compile.lean`) | definition / **proven** |
| Source typing (bytecode-verifier discipline) & type preservation | `IR.WfProg`/`WfInstr`/`WfOp`/`TypedLocals` (`IR/TypedCode.lean`); preservation fused into `Prover.Translate.sim_aux` | definition + **proven** |
| Abstract call over-approximation | `Prover.Translate.contract_call_overapproximates` (`Prover/Translate/Sim.lean`) | **proven** |
| Forward simulation (block-wise) | `Prover.Translate.sim_aux`, `compile_simulates` (`Prover/Translate/Sim.lean`) | **proven** |
| End-to-end prover soundness | `Prover.Translate.prover_sound`, `funExec_conforms` (`Prover/Translate/Adequacy.lean`, `Sim.lean`) | **proven** |
| Account example (TACAS'22 Fig. 1/2) | `Examples.Account.withdraw_verified` | **proven** |
| Loop example (branch, back edge, invariant rule) | `Examples.CountDown.count_down_verified` | **proven** |
| Account example, authored in masm through the frontend | `Examples.MasmSource.masm_withdraw_verified` | **proven** |
| Loop example, authored in masm through the frontend | `Examples.MasmSource.masm_count_down_verified` | **proven** |
| Loop example, authored in Move source through the frontend | `Examples.MoveSource.count_down_verified` | **proven** |
| Global-memory example, authored in Move source | `Examples.MoveSource.take_verified` | **proven** |
| Borrow-based account example: executes, eliminates, verifies | `Examples.BorrowAccount.borrow_withdraw_verified` (+ `#guard`-pinned eliminated code and interpreter agreement) | **proven** |
| Cross-call borrow example: a caller verified *through* the callee's contract (`callRel`'s abort branch refuted by `requires` against `aborts_if`, the final pinned by `ensures`) | `Examples.CrossCall.inc_verified`, `bump_verified` (+ pinned `refElimProg` output and interpreter agreement) | **proven** |
| Frontend elimination pipeline: borrow-based source verified directly | `Frontend.MProgram.elim` (`Frontend/Elim.lean`), the `masmElim%`/`moveElim%` elaborators; `Examples.ElimSource.bump_verified` — borrow-based Move source against its genuine `spec` block | definition + example **proven** |

### `sorry` inventory

Exactly one declaration is stated but not yet proven (it fixes the
architecture and is the next proof target):

1. `IR.refElim_correct` — `IR/RefElim.lean`

This is the **only** sorry in the tree: the simulation and adequacy
theorems (`sim_aux`, `contract_call_overapproximates`,
`compile_simulates`, `funExec_conforms`, `prover_sound`) are proven, under
the source-typing hypothesis `WfProg` (`IR/TypedCode.lean`) — the
discipline the bytecode verifier guarantees for the code the prover
verifies, without which contract facts of opaque calls (stated over
well-typed boundaries) cannot be transported.

### Semantic fine print

- `aborts_if` biconditional: `SatisfiesContract` uses the *per-execution*
  form (normal outcome ⟹ the abort condition evaluates to false; abort
  outcome ⟹ it holds), which is what the Fig. 8 exit assertions check.  For
  deterministic, terminating Move this is equivalent to "aborts iff cond";
  the existence direction would need a termination argument, which
  assert-based verification does not provide.  A function **without**
  `aborts_if` clauses makes *no claim* about aborts (`Contract.aborts =
  none`) — the prover's partial default; an explicit `aborts_if false`
  forbids aborting.
- `==` on references compares the values referred to, as in the Move VM:
  `Oper.sem`/the interpreter dereference top-level reference operands
  (stuck if unresolvable).
- Arity discipline: an operation or call whose result count does not match
  its destination count is *stuck* (no silent truncation), and values
  stored to global memory or packed into structs must be reference-free
  (`Value.refFree`) — as the Move type system guarantees.
- Ill-typed configurations (branching on a non-boolean local, calling an
  undeclared function, ill-typed operands, …) are *stuck* in the source
  semantics — no outcome, hence trivially represented.  Types are
  *declared* (`Ty`; `FunDecl.locals`/`returns`, struct field types in
  `Program.structs`) but not enforced by the semantics.  Instead, the
  **multisorted discipline** of the prover's Boogie encoding is modeled by
  `IsValid` (the `$IsValid'T'` counterpart, `IR/Typing.lean`):
  `compileFun` assumes well-formedness of the arguments and of global
  memory at entry (`typedEntry`), loop havoc leaves declared targets
  defined and well-formed (havoc ranges over the sort), and opaque calls
  return well-formed results.  Quantifier binders carry their declared
  domain type, and `EvalSpec` bounds the range to the `IsValid` values of
  that type — the counterpart of Boogie's typed quantifiers plus the
  `$IsValid'u64'` range guard (there are no type-test predicates in the
  formula language; typing is never encoded in formulas).
  `SatisfiesContract` is accordingly stated relative to well-typed
  boundary states, and the loop-target completeness side condition
  (`WfProgram`) embodies type preservation of the source.
- Partiality of spec evaluation (`global` of an absent resource, division
  by zero) means *stuck*, and the connectives short-circuit, so the guarded
  idioms `exists<R>(a) ==> P(global<R>(a))` are defined.
- All runtime aborts carry one fixed code (`runtimeAbortCode`); contracts do
  not constrain abort codes.
- The reference operations (`Oper.borrowLoc` … `freezeRef`) **execute**:
  references are runtime values (`RefTarget`: a root local or global
  location plus a path of field offsets and vector indices), with
  read/write-through rules in the big-step semantics and the interpreter.
  They still compile to failing assertions — as in the real prover,
  verification requires the bytecode-level *reference elimination*
  (`refElimFun`), which mirrors the real pass pipeline
  (`eliminate_imm_refs` → `borrow_analysis` → `memory_instrumentation`):
  immutable references become values; a liveness and a borrow-graph
  dataflow (union join at merge points) drive the rewrite into the
  *mutation algebra* (`Value.mut`, the `$Mutation` of the Boogie
  prelude): borrows check out the referenced value into a
  location-carrying mutation, reads and writes act on the carried value,
  and each borrow *death* writes the mutation back along its borrow-graph
  in-edges — into the parent mutation, the root local, or global memory.
  Several candidate parents (a reference joining from different
  derivations) dispatch dynamically via `is_parent`/`is_mut_loc`/
  `is_mut_global` guards in split-off blocks; deaths on only some
  successor edges of a branch write back in edge-split blocks.  Split
  blocks are appended above `body.size` (existing ids and loop headers
  are stable), inherit their source block's loop memberships, and loop
  `valTargets` gain everything the inserted code writes.

  The transformation remains partial; the rejected fragment is exactly
  what Move's borrow checker rules out, plus the cross-function cases of
  a later stage: nested references; `&mut`-typed parameters, returns,
  call arguments and results; exclusivity violations (using or writing a
  local while it is mutably borrowed, re-borrowing while derived
  references live, touching a resource type or calling a function while
  a global borrow on it is live — `move_to` and `exists` are permitted).

  Writes through references reach global memory only at the borrow death
  (the read-update-write cycle that makes the encoding alias-free), so an
  execution aborting while a global borrow is live carries an abort
  memory without the pending mutation.  The forward-simulation
  correctness statement accounts for this: `refElim_correct` (stated,
  `sorry`) relates outcomes by `AgreeOutcome` — normal returns agree
  exactly, aborts on the code — which is faithful to the prover's model
  (`aborts_if` is evaluated in the pre-state) and to the VM (an aborting
  execution discards its effects).

  References **cross calls** by checkout (`RunFrom.callRefOk/Abort`,
  mirrored by the interpreter): a `RefTarget` rooted at a local names a
  slot of the *caller's* frame, so the callee receives it re-rooted at a
  fresh shadow slot above its declared locals, preloaded with the
  referenced value; the callee runs against a `ret`-extended view of its
  body (`Cfg.extendRets`) so the shadow finals surface in the ordinary
  outcome, and the caller writes them back through the argument targets
  and re-roots returned references (a shadow-rooted return is a continued
  borrow of the corresponding argument).  Global-rooted references pass
  verbatim — memory is shared.  This is MVP's `Mut` threading
  (`call r := f(r)` in the Boogie encoding) promoted to the source
  semantics, and it is observationally faithful for borrow-checked code:
  locals are invisible in frame outcomes, exclusive borrows make the
  deferred write-back unobservable, and aborts correctly discard it.
  Well-typed calls (`WfInstr.callFun`) never pass reference values
  (`IsValid.not_ref`), so the checkout rules are invisible to the proven
  simulation — `sim_aux` discharges them by contradiction.

  The *elimination* mirrors the checkout boundary through **borrow
  summaries** (`refElimProg`): `&mut` parameters stay `&mut`-typed but
  hold mutation values (`IsValid` at `&mut` is the mutation shape), every
  `ret` is extended with the parameter finals — existing `result` indices
  are stable, and contracts refer to a final as
  `result (numReturns + k)`, reading payloads with `mutVal` — and a
  returned reference re-enters the caller's borrow graph along the
  callee's summarized derivations (hyper-path edges, dispatched
  dynamically by `is_parent` at the write-back).  Still rejected: calls
  while a *global* borrow is live (the callee's memory footprint is
  unknown), and returned references whose derivation passes through a
  *written* intermediate borrow (`EmitSt.written`; unwritten alias
  intermediates are skipped harmlessly — this is stricter than MVP, which
  silently drops such intermediate payloads).
