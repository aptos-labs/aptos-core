# Higher-order functions

Status: H1–H3 implemented (2026-09-30); H4 in progress (decisions taken 2026-10-01, below); H5 open. Open work is listed in
[`roadmap.md`](roadmap.md).

Move function values (language 2.2) are closures: a function, a mask saying
which of its parameters are bound, and the bound values. Their
specifications (language 2.4) use behavioral predicates (`requires_of`,
`aborts_of`, `ensures_of`, `result_of`) and state labels. This design carries
them through LIR, execution, the denotation, and verification, and connects
them to Move sources, Move bytecode, and Rust function pointers.

Sources: the Move Book (`functions.md`, "Function Values";
`spec-compositional.md`), the Prover's paper
(`move-prover/doc/higher-order-paper-26`) and implementation
(`bytecode_translator.rs`, `mono_analysis.rs`, `spec_instrumentation.rs`),
the bytecode instructions (`file_format.rs`: `PackClosure`,
`PackClosureGeneric`, `CallClosure`; `function.rs`: `ClosureMask`), and
`mono-move/docs/closure_design.md`.

## Move semantics

- A function type `|T1, …| R has A` has abilities from `copy`, `drop`,
  `store`. Naming a function gives `copy + drop`, plus `store` when the
  function is persistent (public, entry, or `#[persistent]`).
- `PackClosure(f, mask, c1..cn)` binds the parameters of `f` whose mask bit is
  set, in any positions, to the captured values; its type is the function
  type of the remaining parameters. Capturing moves the values. A closure's
  abilities are `copy + drop` intersected with those of every captured value,
  plus `store` if `f` is persistent (a partial application of a persistent
  function keeps `store`).
- `CallClosure` consumes the closure and calls
  `f(mask.compose(captures, arguments))`.
- References cannot be captured. Lambdas are lifted to functions by the
  compiler before any backend sees them.
- Equality and ordering compare the module, the function name, the type
  arguments, the mask, then the captured values (`values_impl.rs`,
  `LazyLoadedFunction::cmp_dyn`), never behavior.
- Calling back into a module already on the call stack locks that module's
  resources: resource operations on them abort. `#[module_lock]` locks every
  re-entered module. Neither the Prover nor MonoVM models this today
  (`mono-move/runtime/src/interpreter.rs`, the reentrancy TODO).

## Starting point

What the stack had before H1:

| Layer | Present | Missing |
|---|---|---|
| LIR | `Ty.function` with abilities; `CallKind.closure`, `invoke`; `RuntimeValue.closure handle captures`; behavioral predicates as `SpecOperation.behavior kind range` | capture mask (captures are a parameter prefix); the closure's type instantiation |
| Validation | target, arity, and capture types checked; the function type's abilities ignored | reference captures, abilities, persistence; `invoke` in the borrow analysis |
| Runtime | big-step rules, interpreter, soundness/completeness | `invoke` runs under the caller's type instantiation and registers no returned loan; closures ordered by handle index |
| Denotation | direct calls; `propheticMeaning` defined for any handle | no function `NTy`; closures and `invoke` are `notCarried` |
| Contracts | LeanerLang parses predicates and `@n` labels | contracts reject behavioral predicates |
| LeanerLang | `Fn(..) -> R has ..`, `function[F](target, captures)`, `invoke(f, args)` | masks, generic targets, `f(x)` application |
| Move frontend | function types, `invoke`, predicates in the exchange | closure construction: the producer skips the function, and `Encode.lean` drops the skip without a diagnostic |
| XIR | — | function types, `PackClosure`, `CallClosure` |
| Rust | function pointers (non-capturing) | capturing closures, `Fn*` traits |
| Tests | `Tests/Closures.lean`, `Tests/Generics.lean` | Check, SourceVerify, and MonoVM differential fixtures |

## LIR

**Masks.** `CallKind.closure` gains the mask, a bit set over the target's
parameters as in Move; a prefix is the mask with the low `n` bits set. The
alternative, adapter functions that reorder parameters, would add functions
the source does not have and break the LeanerLang round trip. `compose` and
`extract` are defined once in `LeanerIR` and used by validation, the
runtime, and the denotation.

**Instantiation.** A closure of a generic target fixes its type arguments at
construction. `RuntimeValue.closure` carries the handle, the mask, the
captures, and the instantiation that `callTypeInstantiation` computes from
the constructing frame; `invoke` calls under that instantiation, not the
caller's.

**Validation.** For every profile, until Rust closures need more:

- no captured type contains a reference;
- the function type's abilities are within those the captures and the
  target allow (above); Move's persistence rule is a Move profile rule
  (`ProfileSchema.storableTarget`, defined once as
  `LeanerLang.moveStorableTarget` over both spellings of the modifiers and
  used by the LeanerLang and Move registries, `LIR-CLOSURE-STORE`);
- `invoke` requires the operand's function type, arity, and argument types
  (exists), and consumes a non-`copy` function value;
- the borrow analysis treats `invoke` as a call: reference arguments are lent
  for its duration, and a returned reference is a loan of the invocation
  site, registered as for a direct call.

**Values.** `ValueHasType` for a closure requires the target's signature,
instantiated and with the mask applied, to match the function type, and the
captures to have their parameter types. Closures compare as the VM compares
them, by qualified function name first; the runtime reads names through the
unit, not handle indexes.

## Runtime

`closureValue` evaluates the captures and builds the closure with its mask
and instantiation. `invoke` composes captures and arguments by the mask,
calls the target under the closure's instantiation, and registers a returned
reference as `callReturned` does. The interpreter follows the rules; the
soundness, fuel, and completeness proofs extend case by case.

The re-entrancy lock needs the modules on the call stack in the state and a
locked set that resource operations consult. It lands with MonoVM's own
support (H5); until then the semantics, like the Prover's, does not abort on
re-entrant resource access, and MonoVM differential fixtures avoid it.

## Denotation

A function value denotes its closure: `NTy.function parameters results` has
the carrier `ClosureValue` (handle, mask, instantiation, and captures as
runtime values that hold no loan), the same data the runtime holds, so
equality, ordering, storage in structs, and passing are those of any value.
One carrier serves every function type; the type is a typing fact about the
closure, as for other runtime-represented carriers. Runtime values have a
lawful structural equality and a decided plainness, so the carrier's
equality and codec are computable.

- `Term.closure handle weave results captures` builds the closure of a
  target without type arguments, `Term.closureGeneric` one with them; the
  `Weave` relates the target's parameter row to the captured and supplied
  rows position by position, and the compiler checks that its mask is the
  program's.
- `Term.invoke shape f args` denotes `closureMeaning unit f σs shape args`:
  the prophetic meaning of the closure's target under its instantiation,
  with its captures composed into the lent arguments by its mask.

A known closure — its target, weave, and captures in view — invokes as a
call of its target (`Denote/Closures.lean`): `closureMeaning_closureOf`
equates the invocation with the target's `propheticMeaning` on the woven
arguments, since captures hold no reference and lend as their encodings; for
a generic target, `propheticMeaning_subst` carries the meaning at the
caller's types to the family the type arguments induce, the form a direct
call with type arguments has. The normalizer applies both (the generic one
through a simproc, since the invocation spells the substituted rows), so
the closer uses the target's theorem as for a direct call, and closure
targets join a function's callees.

How this relates to the Prover's datatype of function-value sources is the
subject of the next section.

The agreement axiom (`Agreement.lean`) extends to the new terms; since
`invoke`'s denotation is defined from the big-step meaning, their case of the
eventual agreement proof is by definition.

## Relation to the Prover's encoding

The Prover represents the function values of a type `τ` by a datatype of the
program's sources (`encoding.tex`, "Representing Function Values"): a
constructor `C_f(c̄)` per closure site, `P` per function-typed parameter of the
verification target, and `F(n)` per function-typed struct field. The datatype
does three things:

1. **First-order representation.** SMT has no function values. The
   constructors stand for them, and uninterpreted per-variant predicates
   (`requires_P`, `aborts_P`, `result_P`) stand for an unknown function's
   behavior, tied together by axioms: the one-directional result axiom, and
   `ensures_P` defined as the graph of `result_P`.
2. **Dispatch.** `invoke_τ` branches on the constructor: a closure calls its
   function, and a parameter or field goes through the behavioral predicates.
3. **Closed world.** `τ` has no other values, so a verification condition
   sees only the function values derivable from the program, and a value
   whose source it cannot see is still one of the program's closures.

Lean is higher order, which removes the need for the first, keeps the second,
and requires the third to be stated as a theorem.

**Representation.** A Lean proposition can quantify over closures and state
the meaning of an arbitrary one, so a function value is its closure, and the
behavioral predicates are definitions over its meaning (next section) rather
than uninterpreted functions with axioms. What the axioms had to get right
holds by construction: `result_of` is the result the function computes, since
the big-step relation is deterministic, and a target without a specification
has exact predicates rather than a vacuous default.

**Dispatch.** The schema stays, applied to terms instead of constructor tags.
At an invocation, a literal closure is `C_f`: it reduces to the direct call. A
parameter is `P` and a value read from a field is `F`: their predicates are
opaque atoms, constrained by the contract's `requires` and by data
invariants. The closer's discipline mirrors the Prover's triggers: a
predicate is unfolded only for a literal closure, through its target's
theorem.

**Closed world.** During an execution of the verified function, a value of
`τ` comes from a closure site that the execution runs, from the arguments,
directly or nested in data, or from the initial state; a callee's results
derive from the same three. The Prover's datatype has `C` variants for the
sites in the verification root's slice, `P` only for parameters of function
type, and `F` only for fields of function type. Every other value is forced
into the closed world, and two probes against the current Prover verify
specifications that the same code violates at run time (aptos-core #20650):

- `apply_first(fs: vector<|u64|u64 has copy + drop>, x: u64)` invokes
  `fs[0]` and verifies `aborts_if false`. The element has no abstract
  variant, and the root's datatype holds only a placeholder constructor
  that the Boogie backend adds to a datatype without variants; invoking it
  is modeled as never aborting. A unit test passing an aborting closure
  aborts.
- `use_make(x)` constructs an `add2` closure and invokes the closure an opaque
  `make()` returns, which is `inc`. `ensures result == x + 2` verifies: the
  root's slice holds only `add2`'s site, so the returned value is taken to be
  it. At run time the result is `x + 1`.

In Lean the closed world is a theorem, and the one that holds is: every
closure an execution creates is a closure of a site of the executable unit;
values from the arguments, however nested, and from the initial state are
arbitrary, where a type without `store` cannot occur in the initial state. It is proved once, by induction over the big-step relation. The
closer uses it on demand, where a proof cannot see a value's provenance, for
example a closure returned by an opaque callee whose contract does not
describe it: the value is one of the unit's closures of `τ`, a case split
whose cases are direct calls, or it came from an input and is known through
contracts only. For `use_make` the unit's sites give `inc` and `add2`, and
the postcondition fails, as it should. Under the modular discipline a
callee's contract describes the closures it returns, so the case split is a
fallback rather than the representation.

## Behavioral predicates and state labels

Three predicates are defined from the closure's meaning, exactly and without
axioms; the fourth from its target's contract. The meaning runs the
invocation on runtime values from a start with the globals of `S` that lends
nothing (`StartsAt`), as a call's prophetic meaning and `closureMeaning`
start (`Proofs/Behavior.lean`):

| Predicate | Definition at `x`, pre-state `S`, post-state `T` |
|---|---|
| `aborts_of<f>(x)` | the invocation aborts from a start at `S` |
| `ensures_of<f>(x, r)` | the invocation returns `r` from a start at `S`, leaving the globals of `T` |
| `result_of<f>(x)` | results the invocation returns from a start at `S` (`ensuresOf_resultOf`); unspecified when it returns none |
| `requires_of<f>(x)` | the target's declared precondition at the captures and `x`, read from the module's table (`RequiresTable`); true for a target no closure of the module has |

The big-step relation is deterministic from one state, but starts at `S`
differ in their loan bookkeeping; that they return the same results is not
established, so `ensures_of<f>(x, r)` does not yet give
`r == result_of<f>(x)`.

A contract states partial correctness: it constrains the runs that return
and the aborts, not that a run exists. `result_of` of a known function
value is therefore its target's result only where the invocation returns,
which `¬aborts_of` does not give. The theorem of a function whose
contract, or a callee's at any depth, states `result_of` takes the
hypothesis `Terminating` (runs of the unit's functions end, the Move
Prover's assumption), under which an invocation that does not abort
returns (`returns_of_terminating`); the closer reads `result_of` of a
literal closure that way, and the target's contract then applies to the
run.

`requires_of` names a contract: the meaning of a Move function has no
precondition, but a caller needs the target's precondition to use the
target's verified contract. The table maps each target of the module's
closures to its `requires` over runtime arguments, each parameter read from
its argument (`buildDeclaredRequires`), with one `lir_denote_norm` equation
per target; a precondition stating a behavioral predicate would read the
table itself and is rejected. A verified target's theorem (`Satisfies`)
then yields what the Prover asserts as compliance: under `requires_of`,
`ensures_of` implies the target's postcondition and `aborts_of` its
`aborts_if`.

**Literal closures.** Of a closure whose target, weave, and captures a leaf
sees, on reference-free arguments and result, the predicates reduce to the
target: `requires_of` to its table entry at the woven arguments
(`requiresOf_closureOf`, the closer's `behavior` step), and `ensures_of`
and `aborts_of` to runs of its prophetic meaning (`ensuresOf_closureOf`,
`abortsOf_closureOf`). The closer reads the runtime arguments back as
natives, certifying an integer by its bounds, establishes the target's
precondition, and adds what the target's theorem states of the run
(`ensuresOf_closureOf_verified`, `abortsOf_closureOf_verified`). A target
a call inlines is passed to the closer by its theorem as well. A target
without a theorem, a function without a specification, is read by its
body (decided 2026-10-02, as the Move Prover infers the specification of
such a lambda): a hypothesis stating `ensures_of` or `aborts_of` of it
becomes the weakest precondition of its prophetic meaning under which the
leaf holds of every run or abort (`forall_ok_of_wp`, `of_aborts_of_wp`),
which the call rule inlines. The aborts of a target the proof inlines are
read by its body even where it has a theorem, since a specification without
`aborts_if` does not state them, as the Move Prover derives them from the
body. Where a leaf reads `result_of` of a literal closure at native
arguments and knows neither that the invocation aborts nor that it does
not, it is decided case by case on the abort: an abort is decided as above,
and otherwise the run `result_of` reads applies. A parameter of a shared
reference type is the value it observes: a closure over such a target is
typed at the row of the referents (`ClosureTypedAt`, `observedType`).

Mutable-reference arguments follow the prophetic model, in the Prover's
argument layout (`spec_translator.rs`, `bytecode_translator.rs`): a
predicate's inputs are the arguments' values, a `&mut` one's its entry
value; `ensures_of` takes after them the declared results and then the final
value of each `&mut` argument, in order (`ensures_of<f>(old(x), x)` for
`f: |&mut T|`); `result_of` names the declared results alone, and
`requires_of` and `aborts_of` the inputs alone. The meaning (planned, H4d):
the invocation runs with each `&mut` argument lent under a loan of its own,
`.borrow loan entry`, from a start whose bookkeeping lends exactly those
loans, and the final value is the one the run exports for the loan
(`RuntimeState.pending`), with the holes of returned references filled, as
`argumentsResolve` reads a call; for a literal closure this is a run of the
target's prophetic meaning at the reference `(entry, final)`, so the
closure rules generalize from reference-free rows to rows whose references
are the target's `&mut` parameters.

**Frames.** Without `modifies_of`, Move treats a function parameter as not
modifying global memory, and the compiler checks every closure passed for it.
Here the frame becomes a precondition of the higher-order function: `f` may
change global memory only within the declared resources and addresses.
Callers discharge it from the target's contract frame. `reads_of` is left out
(below).

Implemented for parameters without `modifies_of` (2026-10-03): the theorem
of a verified body with a function-typed parameter assumes, beside its
typing, that it keeps memory (`KeepsMemoryAt`: every run from typed memory
at reference-free arguments ends in the memory it started from). A run of
such a parameter's invocation ends where it started, and a loop whose body
invokes only parameters it does not reassign keeps its state. A caller
establishes the frame of a literal closure from its target's theorem, where
the target's assumptions and precondition hold at every argument and typed
memory and its frame changes nothing (`KeepsMemoryAt.ofVerified`), or else
from the target's body, an obligation over every argument and typed memory
(`KeepsMemoryAt.ofBody`), as the Move Prover derives a closure's footprint
from its code. The Move frontend reads an invocation of such a parameter as
writing no global memory, so a higher-order function's frame is closed.

`modifies_of<f>(a) R[a]` (2026-10-03) reaches LIR as a parameter frame
(XAST 6, RawUnit JSON 1.3): the theorem then assumes `FramedAt`, the closed
frame but for the targets with the formals bound to an invocation's
arguments, and a caller establishes it the same two ways, a target's frame
lying within it; `modifies_of<f> *` assumes nothing. A run of a framed
parameter's invocation keeps the frame as a hypothesis, a loop invoking such
a parameter does not keep its state, and the frontend reads the invocation
as a write. A frame a hypothesis states of a passed function value
establishes any wider one (`FramedAt.mono`).

Default frames of directly function-typed Move fields are now carried as
implicit data invariants (2026-10-05). `EncodedKeepsMemory` states the frame
over the field's runtime encoding. Construction, mutation, parameters,
results, and stored resources check or carry it; an invocation of a closure
read from storage recovers its frame from `MemoryInvariants.read`. The
compiler and contract generator share the field predicate so packing and
mutation cannot miss the obligation. `StoredFrames` checks construction,
publication, removal, and invocation; `StoredFrameErrors` rejects both
packing and assigning a memory-writing closure. This does not constrain
arbitrary function values supplied as type arguments or nested in vectors.
Explicit struct-field write frames (2026-10-06) use `EncodedFramed` instead:
the frame's formals bind invocation arguments, its resource types are
instantiated in the enclosing nominal's type scope, and its addresses are
evaluated in the invocation's pre-state. Other fields of the same struct
are available to address expressions. A wildcard frame imposes no memory
restriction. The same construction and mutation checkpoints establish the
frame, and opaque results carry it to callers. Source-clause markers are
unwrapped when applying these guarantees. Shared-reference arguments admit
referent-typed specification formals, matching behavioral predicates.
Explicit enum-field write frames still receive an unsupported diagnostic.
All four full suites passed with the focused frame checks. Dependency
interfaces still omit nominal contracts, so an explicit write frame at that
boundary is rejected; importing it as an empty frame would be unsound. The
interface rejection and preservation of owned frames have frontend guards.

**State labels.** A contract is a Lean proposition over states, so a label is
a state variable: `..S |~ ensures_of<f>(x, y)` makes `S` the post-state of the
application, and a label quantified by `exists S in *` or `forall S in *`, or
left free in a clause, is a bound `RuntimeState`. LeanerLang's `@n` anchors
already number them; `Contract.lean` binds them. The Prover's flattening of
intermediate states for SMT is not needed.

**Verification.** In a higher-order function's proof the predicates over its
parameters are opaque atoms. The closer rewrites `invoke` on a variable with
one lemma, `wp` of the call as the predicates, and then works with the
contract's clauses about them. `invoke` on a literal closure reduces to the
direct call, and closure targets join the callees whose theorems a proof
assumes, so a caller of a higher-order function uses the target's contract as
it would for a direct call. Generic higher-order functions need nothing
further: the carrier does not depend on type parameters.

## LeanerLang

Existing forms stay canonical: `Fn(A, B) -> R has copy, drop`,
`function[F](target, captures…)`, `invoke(f, args…)`, the predicates, and
`@n` labels. Additions:

- a mask by position: `function[Fn(u64) -> bool](f, _, c)` binds the second
  parameter; `_` marks a parameter supplied at invocation, and a prefix needs
  none;
- generic targets: `function[Fn(T) -> T](f::<T>, …)`, replacing
  `LEANER-CLOSURE-GENERICS`;
- `f(x)` for a function-typed local, printed back as `f(x)`;
- `modifies_of<f>(a: address) R[a]` in a specification, for the frame above.

Lambdas stay out; lifted Move lambdas arrive as ordinary functions.

## Move frontend and backend

- **Exchange.** The producer (`move-model/exchange/src/dump.rs`) exports
  `Operation::Closure(module, function, mask)` with its type arguments and
  captures as a closure node; the frontend encodes it as `CallKind.closure`.
  Lifted `__lambda__` functions already export as ordinary functions.
  `modifies_of`/`reads_of` declarations of function parameters are exported
  with the function's specification.
- **Silent drop.** A skipped declaration carries an `inline` flag (XAST 5):
  `Encode.lean` passes over retained inline functions only, and any other
  skip reports as an unsupported declaration.
- **XIR.** Version 7 adds the function type
  (`{"function": [[params], [results], [abilities]]}`) and the operations
  `closure`/`closure_inst` (target, mask, type arguments; the captures are
  the operands) and `invoke` (the function value last, as in stackless
  bytecode; LIR passes it first). compiler-v2 loads them as stackless
  `Closure` and `Invoke`; a closure's target is a used, not a called,
  function of its creator.

## Rust

Function pointers already reach LIR as closures without captures. Capturing
closures take their environment by reference (`Fn`, `FnMut`) or by value
(`FnOnce`); the reference cases need captured references, which the
prophetic model can express but this design excludes. They are a later
milestone of the Rust frontend, with the `Fn*` traits specialized as other
traits are.

## Tests

- **LIR:** `compose`/`extract`; generic closures keep their instantiation;
  reference captures and excess abilities rejected; `invoke` lends and
  returns references; closures ordered by name.
- **MonoVM differential** (Move sources): masks in every position, generic
  closures, closures as struct fields, equality, aborts inside a closure,
  `&mut` arguments through `invoke`. The harness has no global state and no
  `std::cmp`, so closures in resources are exercised through XIR on MoveVM
  (compiler-v2's `leaner/closures.lean`) and the order by the LIR tests.
- **Check fixtures** (LeanerLang): a known closure through a higher-order
  function; a higher-order function verified against `requires_of`,
  `aborts_of`, `ensures_of`, `result_of`; `&mut` arguments; a generic
  higher-order function; a stored closure; state labels over two calls;
  a parameter frame violated by a closure (expected error); the two Prover
  probes above, `apply_first` and `use_make`, as expected errors.
- **Prover tests:** the functional `closures/` and `state_labels/` cases,
  once the Move Prover suite runs in a Lean mode.

## Milestones

1. **H1 — closures at run time.** Done: masks (`ClosureMask`, RawUnit JSON
   1.2), instantiations fixed at construction, validation (mask, reference
   captures, capture abilities), `invoke` as a call in the borrow analysis,
   name ordering (`ValueOrders`), the interpreter and its proofs, LIR tests.
2. **H2 — Move path.** Done: closure construction in the exchange (XAST 5),
   the silent-drop fix, the Move persistence rule for `store`, the
   LeanerLang mask and generic-target spellings, XIR 7 lowering, the MonoVM
   differential fixture `MonoDifferential/closures.move` (masks, a generic
   target, struct fields, a higher-order function, aborts, `&mut` arguments,
   equality), and compiler-v2's transactional `leaner/closures.lean`
   (the same through XIR on MoveVM, and a closure stored in a resource).
3. **H3 — denotation.** Done: the function `NTy` over `ClosureValue`,
   `Term.closure`, `Term.closureGeneric`, `Term.invoke`, `closureMeaning`,
   known closures verified through their targets' contracts (generic targets
   by the family transport of the prophetic meaning), the Check fixture
   `Check/Closures/Known.lean`, and the Move source `SourceVerify/closures.move`,
   whose lambdas verify through their targets' specifications.
4. **H4 — specifications.** Behavioral predicates, state labels, and
   parameter frames in contracts; higher-order functions over unknown
   closures; the closed-world theorem and its case split; the Prover's
   closure and state-label tests. In steps:
   - **H4a (done 2026-10-01):** `aborts_of`, `ensures_of`, and `result_of`
     defined over the invocation's big-step meaning on runtime values
     (`Proofs/Behavior.lean`), `requires_of` read from a table of declared
     preconditions, and their translation into contracts: a contract stating
     a predicate takes the executable unit as its last parameter, and one
     stating `requires_of` the table after it; theorems instantiate them at
     the unit they quantify and at the module's table
     (`LeanerLang/Tests/Behavior.lean`). Without state labels a predicate
     reads the function's entry state, and in `ensures` relates it to the
     exit state; labels and mutable reference parameters are rejected
     until H4d.
   - **H4b:** the typing of invocation outcomes (decided: preservation of
     runtime typing), so that an invocation of an unknown closure has no
     undefined outcome. Planned in phases in
     [`static-typing.md`](static-typing.md); Phases 1–4 done (2026-10-01):
     the checker at preparation and the preservation theorem; Phase 5 in
     progress: the invocation rule's core and loan independence done.
   - **H4c:** the closer's rule for an invocation of an unknown closure as
     the predicates, the dispatch of a literal closure to its target's
     theorem, and the closed-world theorem. The dispatch is done
     (2026-10-01, "Literal closures" above, `LeanerLang/Tests/Behavior.lean`);
     the rule for an unknown closure needs H4b, since its `wp` excludes the
     undefined outcome.
   - **H4d:** state labels, parameter frames, `&mut` arguments, and the
     Check fixtures, the two Prover probes among them as expected errors.
     Done (2026-10-03): parameter frames with and without `modifies_of`
     ("Frames" above), shared reference parameters, the Check fixture
     `Check/Closures/Frames.lean`, and generic higher-order functions over
     rows of type parameters (`Check/Closures/GenericHofs.lean`). Open:
     state labels, `&mut` arguments,
     explicit `modifies_of` frames on stored closures, and `reads_of` (below).
     Default frames of directly function-typed Move fields are implemented
     (2026-10-05, “Frames” above); explicit field write frames are diagnosed.
5. **H5 — re-entrancy.** Call-stack modules and resource locks in the
   semantics, with MonoVM.

## H4: decisions

Three choices the H4 implementation depends on, each taken as recommended
(2026-10-01):

1. **An unknown closure's `undefined` outcome.** `wp` of an invocation must
   exclude the outcome in which the target's results do not decode at the
   function type. For a known closure the target's theorem excludes it; for
   an unknown one only a typing argument does, and the big-step semantics
   has no preservation theorem yet (`roadmap.md`, section 5). Either prove
   preservation of runtime typing for `EvalFunction`, which the agreement
   proof also needs, or assume of every function-typed input that its runs
   are well typed, as data invariants of inputs are assumed, and discharge
   it at call sites from the targets' theorems. Recommended: preservation,
   since the assumption would have to be conditioned on `requires_of` and
   fails for targets without a theorem.
2. **`requires_of`.** A closure's target is known only at run time, so its
   precondition is a case split over the targets a proof can name: a
   definition generated per verification unit that maps each verified
   function's handle to its contract's `requires` at the woven arguments,
   and every other target to `True`. The closed-world theorem is what lets
   a proof restrict the split to the unit's closures. Recommended as
   stated.
3. **Contracts that read a unit.** The predicates read `closureMeaning`,
   hence the executable unit, which contracts do not take today. Either a
   contract using a predicate takes the unit as a parameter, which the
   verification theorem already quantifies, or the predicates are stated
   over a unit-independent interface of the closure. Recommended: the
   parameter, since the meaning is the unit's.

`aborts_of`, `ensures_of`, and `result_of` are defined from
`closureMeaning` at encoded arguments (`Domain.encode` in `Contract.lean`),
and state labels bind `RuntimeState` variables; these carry no open choice.

## Closures in memory: a typed carrier

Status: decided 2026-10-04 (user): a typed carrier, typed by construction,
over coherent frames. C1a and C1b implemented.

**Why types come up.** A theorem quantifies over values of their native
carriers. A data type's carrier states its static type — a `u64` is an
integer with its range proof (`SpecInt`), a structure a row of typed fields
— so every quantified value is well typed and no proof reasons about types.
A function type's carrier is `ClosureValue`, a target and captures, which
may name a target of any signature: the one carrier that does not state its
type. So a closure read from memory or passed in is typed only by a
hypothesis (`ClosureTypedAt`), and memory with a function-typed field is not
typed for free (`resourcesTypedCheck` fails for it).

**Decision.** A function type gets a typed carrier, as `u64` has one: the
closures whose target has the type's signature at its frame, with captures
typed. The typing is a static fact about the unit and erased at run time;
nothing is carried by the runtime, and no proof obligation states it. Then
no function value — a parameter, a field, a stored closure, an invocation's
operand — needs reasoning about types, and the rule for an unseen closure
takes its typing from its carrier.

**Typing.** A closure inhabits a function type at a unit in the
denotation's own types (decided 2026-10-04, user): `.function ps rs` holds
the closures whose target's signature at their frame compiles to a captured
row, `ps`, and `rs` (`closureRowsIn?`, the compiler's reading of the
target's types), and whose captures inhabit the captured row. Inhabiting a
native type is decided structurally on the runtime value (`NTy.admits`), a
closure capturing closures recursing into its captures; every encoding of a
carrier value admits its type. A type parameter of the runtime family admits
any runtime value.

Native types erase what semantic types state: a shared reference is the
value it observes. The semantic typing the unseen invocation rule and memory
typing need (`HasType`) is derived from the native one where the unit's
readings agree (`TypesAgree`). A function type keeps which of its parameters
are shared references (`NTy.function parameters shared results`, decided
2026-10-04, user): the values stay the ones observed, but `|&u64| bool` and
`|u64| bool` are distinct types. A nominal type keeps its type arguments as
native types beside its rows (`NTy.struct source arguments fields`, enums
alike; decided 2026-10-04, user), which its rows need not mention: without
them `|Cap<A>|` and `|Cap<B>|` of `Cap<phantom F>` were one native type, and
nearly every Aptos unit declares a phantom parameter (`Table<phantom K,
phantom V>`). So a native type reads as one semantic type, and a closure's
native typing fixes its function type's semantic signature exactly, as
`HasType.closure` requires. Runtime values carry no type arguments, so
admission and the carriers ignore them.

A closure's target may also return references. Closure rows require results
holding no reference (`closureRows?`), so such a closure has no native
typing and its creation is not carried. The fix follows the parameters: a
function type marks which of its results are shared references, the values
being the ones observed (recorded 2026-10-04, user; not scheduled).

A function type as a type argument (`Option<|u64|u64>`) is not carried
either: the frame a generic call induces defaults each parameter to an
inhabitant of its argument (`Carriers.default_instantiate`), which a
function type does not have on its own (`NTy.inhabitable`), its values
being the unit's typed closures. The fix is a default the unit provides, or
a family without defaults (recorded 2026-10-04; not scheduled).

**Families at a unit.** `Carriers` states the closure typing, and
`NTy.carrier (.function ps rs)` is the subtype of `ClosureValue` it types;
the ground family types no closure. The runtime family is built at a unit
(`Carriers.runtime unit`), and so is every frame (`Skolems unit`), so
`Memory`, `Comp`, and `Meanings` take the unit. An executable unit is
indexed by the validated unit it executes (`ExecutableUnit unit`), so a
theorem's frames, memory, and carriers are at the module's unit constant:
closed terms, whose reductions Lean's caches keep (`whnf` caches no term with
a free variable). A frame's closure typing is the runtime family's at the
rows it resolves, as its transports to the runtime family force.

**Typed by construction.** Typing is never a proof obligation. Compiled
code is indexed by its unit (`Function unit`, `Term unit`), and a closure
node carries the proof that its target's signature at the unit splits by its
mask into its rows, which `compileFunction` establishes once by evaluation;
the denotation builds the typed closure from it, checking nothing. Decided
2026-10-04 (user). A closure is created typed from that proof and two facts
its creation takes, as the runtime meaning holds them:

- **The unit's readings agree.** The compiler's native type of every type
  identifier and the checker's semantic type are one type
  (`TypesAgree`), decided once per unit by kernel evaluation. It gives the
  target's signature at the closure's rows.
- **The target's frame is coherent.** See below. A closure of a target
  without type parameters has a coherent frame by definition.

The runtime meaning (`propheticMeaning`) has an outcome only where both
hold, as it has none today at a frame disagreeing on resource types
(`Coherent`), so taking them at creation leaves the agreement as stated.
Decoding a runtime closure (a call's result) checks its typing, so the codec
stays tight. An inhabitant of a function type is a typed closure of the
unit, if one exists (`NTy.inhabitable` decides it), not a default.

**Coherent frames.** A generic function is verified once, at a skolem
frame (D3, [`denotation.md`](denotation.md)). Its type parameters appear
there twice: as the frame's types, and in the runtime type instantiation
that its closures and storage keys carry. The Move Prover has one type per
parameter; a frame is coherent when its two agree: its runtime
instantiation is faithful (`FrameInstantiation`) to semantic arguments that
are its types' semantic types, wherever its types have one. This
generalizes `Coherent`. At a coherent caller, a closure's target frame is
coherent by static typing (`FrameInstantiation.call`), which the agreement
proof carries. No theorem shape changes, and no call owes coherence.

`Coherent` speaks of the types the frame reads, as the runtime reads them:
the types it requires (`requiredAt`, the resource types of its storage
operations and the type arguments of its calls and closures, closed under
its calls), each instantiated as the runtime does and, where it is a native
or resource type, the frame's resolution of it; and its type parameters,
each read at its node through the instantiation as the frame's type for
it. The runtime instantiation maps only types interned in the unit's arena,
and static typing interns exactly the required ones, so a clause over every
type of the namespace would fail at real frames, where the runtime meaning
would then have no outcome. Over the required types and the parameters,
coherence holds at the runtime frame, and a coherent caller's calls and
closures have coherent frames: its required types contain the instances of
its callees' at their type arguments.

A closure's rows at its instantiation are its target's own rows with each
type parameter replaced by its argument, read at the parameter's node
through the instantiation, as the semantic typing substitutes its
arguments (`ClosureTyped`). A closure node of a generic target carries its
target's own rows and that each parameter its signature mentions is its
own and has a node; at a coherent target frame the rows read at the
closure's instantiation are then the frame's resolution of the target's
rows.

**Public theorem.** A function without type parameters is stated at the
empty instantiation, as today. A generic function has no meaning there (its
types stay parameters while the instantiation names none); its public
theorem covers the runs at every faithful instantiation, at the frame the
instantiation induces, coherent by `TypesAgree`. Since C2c it is stated at
every type instantiation a call gives the function (`FrameOf`) and type
arguments `θ` whose induced frame over the runtime family is coherent with
it, its contract read at the arguments' types (`Contract.ofSkolem`,
`satisfies_generic_at`); a caller at concrete type arguments discharges the
coherence by evaluation. C3 derives the coherence from faithfulness.

**Consequences.** Memory typing gets its function case from the carrier,
and the contract's `ClosureTypedAt` assumption on function parameters
becomes the carrier's property.

**Semantic typing from the native (C3).** The runtime's typing
(`HasType`) is what an unseen invocation and typed memory need; carriers
state the native one. Three pieces relate them.

- *The readings agree* (`TypesAgree`). `NTy.TypedAs` relates a native type
  to the semantic type its encodings inhabit: scalars, the unit, tuples,
  vectors, structures and enums at their declarations' field types under
  as many arguments as they bind, a mutable reference to its referent's, a
  type parameter to itself, and a function type to the function type of
  its related rows, a parameter marked shared to a shared reference. A
  nominal type reads at its arguments' readings, so a native type reads as
  at most one semantic type (`TypedAs.unique`). A unit's
  readings agree when every closed type identifier the compiler reads
  natively (`ntyOf`) is `TypedAs` its semantic type, and every function's
  signature is, over its generic environment; decided once per unit by
  evaluation. Both readings commute with substitution (`TypedAs.subst`),
  so agreement at a function's generic environment gives it at every
  instantiation whose arguments agree. A unit whose readings do not agree
  is reported as not memory-typed. Agreement is a fact of the unit, not of
  a frame: a closure's creation takes only facts every real frame has.
- *Faithful closures.* `HasType.closure` types a closure only under an
  instantiation faithful to its arguments (`FrameInstantiation`), which
  the native typing does not check. The native typing of a closure also
  checks it, in its decidable form: the arguments the instantiation gives
  the target's parameter nodes are closed, the instantiation is the one
  the runtime builds for them, and every type the target requires is
  interned at them. A closure the runtime builds at a faithful frame is
  faithful (`FrameInstantiation.call`); the creation of a closure of a
  generic target takes it with the target frame's coherence.
- *Closed frames.* A carrier value's encoding has the semantic type of its
  native type when that type is closed; at a frame, a type parameter
  stands for the frame's type for it. A coherent frame of a faithful
  instantiation resolves each parameter with a node to the native type of
  a closed argument, which is closed; a generic body's proof therefore has
  the semantic types of its parameters' values wherever its frame is
  coherent, and the runtime meaning has no outcome elsewhere.

Then a carrier value of a closed type at the runtime family is typed at
the semantic type its native type is `TypedAs` (`NTy.encode_hasType`,
generalized from scalars): a closure by its native typing, faithful
instantiation, and the readings' agreement on its target's signature.
Memory is typed for every resource type the readings agree on, function
fields included, and an unseen invocation's typing (`ClosureTypedAt`) is
derived from the invoked value's carrier, so contracts no longer assume
it of function parameters.

*The unseen rule from carriers (C3c).* An invocation of a function value a
proof does not see needs the closure's typing (`ClosureTyped`), its
arguments' semantic types (`RowTyped`), and its results' decoding
(`RowDecodes`). The first comes from the value's carrier: at any frame its
encoding is admitted at the frame's resolution of its function type, and an
admitted closure is typed at its target's signature where the readings
agree. The other two are facts of the rows at the frame, and the rule stays
decided at rows the frame resolves to scalars (`ScalarAt`). A function
without type parameters resolves its rows to themselves, decided by
evaluation; a generic function whose function-typed parameter's rows mention
its type parameters assumes that its frame resolves them to scalars, which
a call discharges at its type arguments. A resource type whose values hold
no closure is typed by its reading alone; one holding closures also needs
the readings to agree.

| | Scope |
|---|---|
| C3a | `TypedAs` for every native type, decided by evaluation; resource typing through it (enums, instances of generic declarations). Done 2026-10-04. |
| C3b0 | Function types keep their parameters' sharing natively. Done 2026-10-04. |
| C3a2 | `TypedAs` functional (shared references only as marked parameters, nominal arity, determined function types), unique, and under substitution. Done 2026-10-04. |
| C3b1 | Faithful closures in the native typing; a closure's creation takes its target frame's coherence and faithfulness. Done 2026-10-04. |
| C3b2 | Carrier values typed semantically (`HasType`), closures included (`NTy.hasType_of_admits`). Done 2026-10-04. |
| C3c0 | Native nominal types record their type arguments: one reading per native type in every unit, `TypesAgree` without phantom-free declarations. Done 2026-10-04. |
| C3c | Memory typing's function case; the unseen rule's typing from carriers, at rows frames resolve to scalars (`ScalarAt`); the `ClosureTypedAt` assumption retired. Done 2026-10-04. |

**Milestones.**

| | Scope |
|---|---|
| C1a | The subtype carrier, with the runtime family typing every closure: the mechanics, no change of meaning. Done 2026-10-04. |
| C1b | Families, `Memory`, `Comp`, and `Meanings` at a unit; `ExecutableUnit` indexed by its unit; the generated syntax restated. Done 2026-10-04. |
| C2a | Compiled code indexed by its unit (`Function unit`, `Term unit`): the mechanics, no change of meaning. Done 2026-10-04. |
| C2b1 | The typing of closures of targets without type parameters (`closureRowsIn?`, `NTy.admits`); closure nodes carrying their rows. Done 2026-10-04. |
| C2b2 | Closures of targets with type parameters typed at their frames, frame coherence in the runtime meaning, taken at creation; coherence over the required types and the type parameters. Done 2026-10-04. |
| C2c | Decoding checks typing (by the typed codec since C2b1); generic public theorems at each instantiation coherent with the frame its type arguments induce. Done 2026-10-04. |
| C3 | Semantic typing derived from the native (`TypesAgree`); memory typing's function case; the unseen rule and function parameters typed by their carriers. Done 2026-10-04. |
| C4 | Prover tests `closures/stored_fun_values`, `behavioral_target_two_masks`. |

## Non-goals

- Lambdas in LeanerLang.
- Captured references, for Move and Rust alike.
- `reads_of`: a meaning is exact, so reads need no declaration to be sound;
  what it adds, independence of the result from other memory, can follow.
- `unchanged_of` and `folds_of`, which exist only for lambda arguments of
  inline functions, expanded before export.
- Enumerating closures of code outside the executable unit: such closures
  reach a verified function only through its inputs, which are arbitrary.
