# Direct verification of Lean-authored Move

Status: living design document

This document describes how contracts written with `import Move` can be
specified and verified directly in Lean. The source proof experience is the
primary concern. Compilation still goes through `Move.Compiler.LIR` and
`MoveModel.IR`, and imported `.move` code continues to use the existing
`MoveModel.IR` semantics.

The central representation decision is to erase immutable references to
read-only values and to model mutable references with RustHorn-style
prophecies. References are therefore not locations in the generic `Action`
state.

## Implementation status

The first proof-facing semantic slice is implemented:

- `Move.Semantics.Spec` defines the authoritative relational normal/abort
  semantics and its monadic composition;
- `Move.Semantics.Outcome` and `Txn` provide a deterministic embedding used by
  semantic unit tests, not a source execution engine;
- `Move.Semantics.Checked` defines checked integer arithmetic, shifts, and
  casts, generic over the width, and their lifting into transactions;
- `Move.Semantics.Reference` defines immutable-reference erasure, mutation
  values, prophecy loans, concrete-loan erasure, lenses, and nested reborrows;
- `Move.Semantics.Global` defines typed resource descriptors and scoped mutable
  global checkout/restoration, plus compositional `ResourceStore` views and
  pairwise frame laws for independent resource families;
- `Move.Verify.Contract` defines contracts, satisfaction, fixed-point
  induction, and the wp calculus core;
- `Move.Verify.WP` states one-obligation wp rules for the supported
  primitives, including checked arithmetic;
- `Move.Verify.Borrow` supplies derived prophecy rules for reads, writes, and
  vector mutation through borrows;
- `Move.Verify.Tactics` provides the proof tactics (`contract_intro`,
  `move_step`, `move_hyp`, `checked_cases`, `abort_clause`, `spec_norm`,
  `move_cases`, `wp_call`, `u64_omega`; well-definedness is discharged
  structurally by `spec_defined`).  `verify` proofs are wrapped in an inert
  `move_bench` that, under `MOVE_PROOF_BENCH`, records deterministic
  per-proof heartbeats for `scripts/bench-proofs.sh`.  All share the
  `move_norm`/`wp_norm`/`move_spec` simp inventories used by both manual
  proofs and the automatic `verify`;
- `Move.Verify.Syntax` provides function-associated `spec` and `verify`
  commands. Pure contracts reduce the source function directly. Effectful
  contracts retain the unexpanded `fun` body and generate `f.sourceSpec`,
  `f.contract`, and `f.verified` as Lean-only declarations;
- effectful contracts infer their resource families from the body. Authors use
  `existsAt<R>(addr)`, `old(R[addr].field)`, and post-state `R[addr].field`
  directly; they do not define a `World` or resource descriptors. `verify f`
  performs the standard symbolic proof;
- effectful source generation covers immutable and mutable local, vector, and
  global-field borrows; prophecy-backed reads and writes; vector insertion and
  removal; checked integer arithmetic, shifts, and casts at every width;
  conditionals; structured `while` /
  `loop` / `break` / `continue` (including labels) as fixed-point semantics;
  calls through an Action-returning callee's `sourceSpec`; finite relational
  recursion, including generic recursive functions over vectors; returns; and
  aborts. Calls to pure compiled helpers are rejected until they have an
  equivalent relational summary. Unsupported source forms are rejected at the
  `spec` command;
- ordinary Lean imports make another Lean-authored module's definitions,
  `sourceSpec`s, contracts, and theorems available to the caller proof. The
  executable compiler independently records the cross-module call in XIR.

`Tests.Move.LowLevel.SourceVerification` checks relational contracts, existential
prophecy loans, nested field reconciliation, rollback, typed global mutation,
overflow, and missing-resource behavior. The end-user surface is exercised in
the actual `Tests.Move.Account` and `Tests.Move.Arithmetic` programs: their
declarative contracts prove checked resource updates directly beside the
source and the same functions are then lowered and executed. The separate
`Tests.Move.LowLevel.ModuleVerification` fixture is limited to lower-level semantic
helpers. `Tests.Move.ResourceComposition` verifies and executes a function
which sequentially mutates two resource families, exercising automatic loan
scoping and generated cross-resource frame laws. Existing compiler and
interpreter tests continue to pass.

The pure source modules in `Tests.Move` use the declarative surface directly:

```lean
spec choose (fallback : U64) (choice : Choice) where
  ensures
    result = match choice with
      | .Fallback => fallback
      | .Chosen value => value

verify choose
```

The commands associate the generated declarations structurally as
`choose.contract` and `choose.verified`; neither is selected by Move lowering.

The compiler-facing `Move.Action` primitives remain markers with their
reducible `StateM` shape. The `fun` command also retains their source syntax
for verification, and an effectful `spec` generates the corresponding
relational core directly. The ordered-map benchmark verifies generated
recursive binary search by fixed-point induction, `Tests.Move.Loops` verifies
structured loops, and `Tests.Move.Quicksort` verifies a generic in-place sort
against the semantics derived from its authored body — no example states a
relational `sourceSpec` manually. Automatic global publication/removal,
mutual-recursion SCC semantics, invariant annotations for `continue`-lowered
loops, and some nested-loan shapes remain incomplete. Most importantly, the
compiler-correctness theorem connecting generated `Spec` behavior to LIR
lowering has not yet been proved.

## Goals

- State and prove contracts against the Lean-authored source interface.
- Give source operations their actual Move behavior, including arithmetic
  aborts and transaction rollback.
- Keep the current concise notation: `&`, `&mut`, `*r`, and `r := value`.
- Represent borrows without a heap of reference cells or an alias map in
  `Action`.
- Support local, field, vector-element, and global-resource borrows uniformly.
- Support mutable references across function calls and nested reborrows.
- Connect source proofs to the generated `MoveModel.IR` program with a compiler
  correctness theorem.
- Compose source proofs with summaries for functions imported from Move.

## Non-goals

- Lean's ordinary reduction is not the final semantics of an arbitrary source
  declaration. The compiler-supported subset receives an explicit source
  semantics.
- Lean's type system alone is not expected to enforce Move's affine borrow
  discipline. The compiler validates it and produces evidence used by the
  semantics and correctness proofs.
- The source verifier does not replace bytecode verification.
- Prophecies are logical values. They are not stored in generated Move
  bytecode.
- Lean source declarations do not need a separate concrete evaluator.
  Execution always uses the compiled IR/bytecode path.

## Semantic layers

There are two relevant interpretations of a source declaration:

```text
Lean source declaration
       |
       +-- pure `f` / generated effectful `f.sourceSpec` + `f.contract`
       |                         |
       |                     `verify f`
       |                         v
       |                 kernel theorem `f.verified`
       |
       +-- Move.Compiler.LIR -> MoveModel.IR
                 |                  \
                 |                   +-> interpreter / existing IR prover
                 v
          MoveModel.Frontend.XIR       versioned transport only
                 |
          compiler-v2 stackless and file-format pipeline
                 |
          production bytecode verifier
```

The verification interpretation is relational because a prophecy is a fresh
logical variable constrained by the future execution of a loan. It need not
choose one computable result. The compiler interpretation is the only deployed
execution meaning of the contract.

XIR generation is not part of the source proof and XIR carries neither a
contract proof nor a weakest precondition. Likewise, the production bytecode
verifier proves bytecode safety, not the functional source contract.

The missing main soundness theorem will state that every generated IR
execution is contained in the source verification relation. A contract proved
for every behavior admitted by that relation would then hold for the compiled
program. Representation preservation through XIR decoding is a separate,
smaller serialization boundary.

## Transaction effects

`Action` represents effects which are observable at the transaction boundary:

- global resource reads, publication, removal, and replacement;
- success with a return value;
- abort with a Move abort code;
- rollback of global changes on abort.

Verification uses normal and abort relations over an abstract state type:

```lean
structure Spec (State Result : Type) where
  ok     : State -> Result -> State -> Prop
  aborts : State -> Nat -> Prop
```

The state is not a program-specific `World` record. Contracts quantify over an
abstract state plus `ResourceStore State R` views for each resource family
they mention. Composition adds another view and frame-law assumptions instead
of changing a central state type. The state contains no reference cells,
reference identities, mutable-reference payloads, or alias table.

The abort relation has no final world, so rollback is built into its shape.
This matches the VM boundary and the existing IR agreement relation, which
compares memory only for normal returns.

## Immutable references

An immutable reference is a read-only observation of a value. Its logical
representation is the value itself:

```lean
abbrev Ref (alpha : Type) := alpha
```

Consequently:

- an immutable borrow observes the current owned value;
- `*r` is logical dereference and returns that observed value;
- field and vector-element borrows select from the observed value;
- no write-back is needed when the borrow dies.

This representation does not assert that every Move value has the `copy`
ability. The source borrow checker still prevents an immutable observation
from being used as an illegal owning copy. Erasure is a semantic
representation justified by a valid borrow certificate, not an additional
source operation.

This agrees with `MoveModel.IR.RefElim`: immutable reference locals are
replaced by their dereferenced values before mutable-reference elimination.

## Mutable references and prophecies

The logical shape of a mutable reference is:

```lean
structure MutRef (alpha : Type) where
  current : alpha
  prophecy : alpha
```

`current` is the value checked out by the loan and subsequently updated by
writes. `prophecy` is a fresh logical value denoting what `current` will be
when the loan ends. It is fixed for the lifetime of the reference.

The operations are logically pure transformations:

```lean
def MutRef.read (r : MutRef alpha) : alpha := r.current

def MutRef.write (r : MutRef alpha) (value : alpha) : MutRef alpha :=
  { r with current := value }
```

The surface statement

```lean
r := value
```

therefore rebinds the compiler-managed reference local to
`r.write value`. It does not update a reference heap in `Action`.

### Loan creation and death

If an owned place contains `v`, creating a mutable loan introduces a fresh
logical variable `p`:

```text
borrowed reference:  { current := v, prophecy := p }
suspended owner:     p
```

The owner cannot be used while the loan is live. At the loan's death, the
verification relation adds the constraint

```text
reference.current = p
```

and the owner resumes with value `p`. Thus writes determine the prophecy
rather than obtaining a guessed runtime value. In a relational presentation,
`p` is a fresh variable existentially related to the completed execution.

No runtime operation creates `p`; this source relation is used only in proofs.
Execution reaches the same borrow through compiled IR, where the existing
reference and reference-elimination semantics determine the runtime behavior.

### Borrow scopes

Move surface syntax does not mark the end of each loan. The current source
translator places the remaining `do` continuation inside a scoped
ownership-passing operation, conceptually:

```lean
withMutBorrow place fun reference =>
  body
```

The scope owns the suspended place, introduces the prophecy, and reconciles
the final value. This is compiler-generated core syntax; users retain the
existing `let r <- &mut place` notation.

For the full language, this syntactic scope must be checked against the
compiler's reference liveness and borrow graph (or a shared certificate). A
declaration which has no valid borrow certificate must be rejected rather than
assigned an approximate semantics. That certificate connection is not yet
implemented.

## Nested borrows

Suppose a parent mutation has current value `owner` and prophecy `ownerFinal`.
Borrowing a field through it introduces a fresh field prophecy `fieldFinal`:

```text
child.current      = getField owner
child.prophecy     = fieldFinal
suspended parent   = setField owner fieldFinal
parent prophecy    = ownerFinal
```

At child death, `child.current = fieldFinal`. The parent resumes with the
field prophecy installed at that field. Its own eventual loan death relates
the reconstructed owner value to `ownerFinal`.

Vector-element reborrows use the same rule with a checked element lens. An
out-of-bounds index aborts before a child loan is created. Sibling mutable
borrows are accepted only when the borrow analysis proves their paths
disjoint.

This nesting rule is the source-level counterpart of `Value.mut`, mutation
paths, and deferred parent write-back in `MoveModel.IR.RefElim`.

## Global mutable borrows

A global borrow checks a resource value out of the transaction world and
passes it through a scoped mutation:

```text
World resource at (type tag, address): v
mutable loan current:                v
mutable loan prophecy:               p
suspended global owner:              p
```

On normal loan death, the verification relation restores `p` and constrains it
to the reference's final current value. The resource cannot be observed,
removed, or borrowed in an overlapping way while ownership is suspended.

On abort, pending write-back is unobservable because the transaction world is
rolled back. No mutable-reference payload is exposed in the abort result.

The suspended ownership obligation belongs to the generated loan scope, not
to the generic `World` representation.

## Mutable references at function boundaries

A function taking `&mut T` is interpreted as a relation between its current
input and its future output. Prophecies make this relation compositional:

```text
callee input:   current value
callee output:  value named by the argument prophecy
```

A caller suspends its owner using the same prophecy, invokes the callee, and
resumes with the callee's final value. Multiple mutable-reference parameters
must have distinct or provably disjoint origins. Returned references retain
their derivation relationship and are accepted only where the borrow
certificate can represent that lifetime.

Function contracts should expose value relations rather than internal
reference identities. For example, a contract for `increment : &mut U64 ->
Action Unit` relates the pre-state current value to the post-state prophecy.

Mutual and ordinary calls are verified modularly from these relations.
Explicit `continue f ...` tail calls share the finite-unfolding recursion
semantics of ordinary self-calls; they do not require reference state in
`World`.

`aborts_if` clauses may be repeated; their conditions and exact codes are
disjoined into one abort predicate. The postcondition follows the standard
Move reading `¬ aborts_cond → ensures_cond`: it must hold exactly where the
declared aborts are ruled out. This implication is part of the semantics of
contract satisfaction itself, not text written into the generated clauses,
so `contract_intro` presents each abort condition's negation as a hypothesis
of the `ensures` proof (clauses written `aborts_if False` add nothing).

A specification also frames global memory implicitly. A function changes only
what its `modifies` clause lists, so the contract carries that frame as its
own component — `Contract.frame`, defaulting to "changes nothing" — and no
contract states one by hand.  The frame is owed by every successful
execution, unconditionally: unlike the postcondition it is not excused where
a declared abort may happen.  What the clause generates:

- no `modifies` clause — no global memory changes, expressed on the abstract
  state as `final = initial`;
- `modifies R[addr]` — every other address of `R`, and every other resource
  family the function uses, has the same `ResourceStore.lookup` in the final
  state as in the initial one;
- `modifies R` — the family is unconstrained, and the others are still framed.

Address-level framing needs the store abstraction to know that distinct
addresses are distinct locations, which is why `ResourceStore` carries
`lookup_insert_ne` and `lookup_erase_ne` beside the hit laws.

Abort behavior therefore has two independent components, and a contract
carries both:

- `aborts` — which abort outcomes the contract permits. This is what an
  aborting execution is checked against, and what a caller inherits.
- `mayAbort` — the states in which a declared abort excuses the
  postcondition. This is the disjunction of the declared abort conditions.

For declared conditions the two agree, and `mayAbort` is exactly what the
clauses say. They come apart when a contract declares no abort condition at
all: abort behavior is then *uninterpreted*, so every code is permitted while
nothing is excused, and every successful execution must still establish
`ensures`. Deriving one component from the other would make an omitted clause
silently void the postcondition, which is why both are recorded.

Because `aborts_if` is read as an over-approximation — the function *may*
abort where the condition holds — a contract cannot express that a function
*must* abort. `ensures False; aborts_if True` states that any abort carries
the declared code, not that the function never succeeds.

## Data invariants

A type may declare an invariant with `spec T where <condition>`. Values of
such a type are certified: the generated structure carries the proof as a
field, so reading, passing, storing, and returning one need no obligation and
contracts need no precondition. What states the condition is a generated
proof-free twin `T.Raw`, since the certified type cannot exist before its own
condition; `this` is a value of that twin, which is what lets an enum
invariant match on it.

Creation is the only obligation, in two forms. A literal discharges it during
elaboration through the field's `by move_invariant` default. Re-establishing
it after a mutation cannot be discharged there — it depends on run-time values
— so the translator emits `Spec.certified`, whose `undefined` component is the
negated condition; `wp_certified` turns that into a positive obligation at the
point the loan dies. This is why `Spec` carries well-definedness at all: a
rule whose `ok` relation were merely empty on violation would make `wp` hold
vacuously.

Using the invariant is unobligated but not automatic. A proof that needs the
condition as a *fact* — rather than as an argument to a model lemma, which is
how most of them use it — calls `data_invariants`, which asserts the unfolded
condition of every certified-typed local without naming the type or its
generated predicate. This is deliberately a tactic rather than part of
`uint_bounds`: a representation bound is one cheap atomic fact, while a data
invariant can be an arbitrarily large predicate, and asserting one into every
context the automatic cascade normalizes measured as a net loss across the
suite (see `performance-analysis.md`).

## Checked Move operations

Source operations must have Move behavior before direct proofs are useful.
In particular, integer arithmetic is bounded and partial at every width:

- addition and multiplication abort on overflow;
- subtraction aborts on underflow;
- division and remainder abort on zero;
- shifts abort when the `u8` amount reaches the width's bit count, and left
  shift truncates shifted-out bits;
- casts abort when the value does not fit the target width;
- division overflow rules follow the corresponding Move integer operation.

The familiar notation remains available. The source elaborator or compiler
effect-lifting pass maps `+`, `-`, `*`, `/`, and `%` to checked operations in
an `Action` context. Proof lemmas expose both the success condition and abort
condition without forcing users to unfold implementation details.

The source operators in `Move.Basic` remain named compiler markers, but are
reducible to their mathematical value equations for proofs. Effectful source
generation maps them to `Semantics.Checked`, so overflow, underflow, and
division by zero are represented as aborting behaviors rather than Lean
reduction results.

## Contracts

The semantic foundation remains ordinary Lean definitions and theorems. A
minimal relational contract type is:

```lean
structure Contract (State Args Result : Type) where
  requires : Args -> State -> Prop
  ensures  : Args -> State -> Result -> State -> Prop
  aborts   : Args -> State -> U64 -> Prop
```

`Satisfies f contract` states:

- if `requires args initial` holds and `f args` returns normally, `ensures`
  relates the initial and final worlds and return value;
- if it aborts, `aborts` permits the code;
- committed state is still the initial state on abort.

Effectful source verification uses the same associated notation. The state
type, typed resource views, and pairwise frame laws are inferred and
universally quantified rather than declared by the contract author:

```lean
spec deposit (addr : Address) (amount : U64) where
  requires existsAt<Balance>(addr);
  ensures
    Balance[addr].balance.value =
      old(Balance[addr].balance.value) + amount;
  aborts_if
    ¬old(Balance[addr].balance.value).toNat + amount.toNat < U64.size
    with Semantics.Checked.arithmeticAbortCode

verify deposit
```

For pure source functions, the implemented declarative notation associates the
contract and theorem with the source function:

```lean
spec classify (action : Action) where
  ensures result = match action with
    | .Idle => 0
    | .Transfer _ => 1

verify classify
```

The generated `deposit.sourceSpec` interprets the retained source statements
with `Semantics.Spec`, checked arithmetic, typed resource operations, and
prophecy mutations. The generated `deposit.contract` quantifies over an
abstract state and `ResourceStore` instances. When several resource families
occur, generated `IndependentResourceStores` assumptions provide the frame
laws needed to preserve the other families across insertion and removal.
`initial`, `final`, `result`, and `abortCode` remain available as implicit
clause binders, but the global-place notation normally hides the state names.
The Move-style `aborts_if P with C` form expands to `P ∧ abortCode = C`;
omitting `with C` constrains only the condition and permits any code.
The `Move.Spec` syntax scope is separate from `Move`, avoiding conflicts with
the fields of the lower-level relational contract.
Specifications cannot expose reference identities or prophecies; prophecies
remain an internal proof device.

## Verification interface

The implementation exposes compositional relational/WP rules for:

- pure return and monadic bind;
- normal and abort outcomes;
- typed global resource operations;
- checked arithmetic;
- immutable observation;
- mutable loan creation and death;
- writes and freezes;
- field and vector-element reborrows;
- calls from source contracts or imported summaries;
- conditional branches and finite recursive unfoldings.

The mutable-loan WP rule introduces a fresh symbolic final value, verifies the
scope with a `MutRef current final`, and reconciles the scope's final current
with that value. User proofs should normally see the derived read, write, and
borrow lemmas rather than this primitive rule.

The implemented `verify f` command opens the contract with `contract_intro`
into one weakest-precondition goal and executes the body symbolically by the
`wp_norm` rules (`simp only`, linear in the body, no existentials, a
prophecy eliminated as soon as its reconciliation equation appears), then
finishes the arithmetic and data obligations with the shared `move_spec`
and `move_data` inventories,
simplifies typed resource lookup and prophecy equalities, and leaves
domain-specific arithmetic or data-structure obligations to ordinary Lean
tactics. `Move.Verify.satisfies_fix` — with its wp form `satisfies_fix_of_wp`,
packaged as the `contract_intro` tactic — supplies fixed-point induction for
direct recursion and structured loops. More advanced automation for mutually
recursive SCCs and invariant-driven `continue` loops remains future work.

Every checked operation has the same two-branch weakest precondition — a
success condition and an abort with a fixed code — so one tactic splits them
all. `checked_cases h` normalizes the leading checked operation of a wp goal,
names the branch condition `h`, and discharges the abort branch against the
contract's declared clauses; `abort_clause` does that discharge alone. The
observed abort code selects the matching clause, arithmetic proves its
condition, and a branch no clause admits is refuted from the context. A
condition needing a semantic argument (a model-level `Contains`, say) is left
as the only remaining goal, so the tactic never hides real proof obligations.
`Move.Verify.wp_withMutation_insertSpec` and `wp_withMutation_removeSpec` put
vector insertion and removal through a mutable borrow into that same shape.

`<` and `==` in retained source denote Move's sealed structural comparison
markers uniformly (`logicalLT`/`logicalBEq` are the marker relations
themselves). Two axioms, `Move.Verify.Source.logicalLT_uint` and
`logicalBEq_uint`, state the compiler semantic fact that the comparison
instructions are numeric on the integer types, at every width; a third,
`Move.UInt.less_eq_true_iff`, states the same for the integer-specific
`UInt.less` primitive behind the `<` instance of `UInt` in ordinary (pure)
function bodies. They are the
verification interface for the compiler-implemented ordering and an explicit
part of the trust base; the generic bridges `logicalLT_move`/`logicalBEq_move`
under `[Move.Compare.Total T]` are definitional.

## Accepted-program evidence

Lean values are structurally copyable unless their API prevents access, so
the source representation alone cannot establish Move borrow legality. The
compiler must produce or check evidence including:

- reference locals are not duplicated illegally;
- mutable loans do not overlap except at proven-disjoint paths;
- suspended owners are not read, moved, or overwritten;
- parent mutations are not used while a child is live;
- reference results do not outlive their roots;
- joins and calls preserve a valid borrow graph;
- every loan has well-defined death points on normal control-flow edges.

A first implementation can use a decidable `WellBorrowed` predicate. The
longer-term result should be a certificate produced by analysis and validated
by a small Lean checker. Source semantic theorems should take this evidence
explicitly or obtain it from a generated module certificate. The present
source translator handles its accepted syntactic subset without this shared
certificate, so source proofs do not yet establish bytecode borrow legality.

## Relation to `MoveModel.IR`

Compilation continues through the established pipeline:

```text
Move source declarations
  -> Move.Compiler.LIR
  -> MoveModel.IR
  -> MoveModel.Frontend.XIR
  -> compiler-v2
  -> Move bytecode
```

Direct source verification is independent of this compilation path: it neither
bypasses nor consumes `MoveModel.IR`, and it does not consume XIR. The intended
compiler-correctness theorem will relate the two branches at the source-to-IR
boundary.

The desired refinement result is:

```text
MoveModel.IR execution  refines  source prophecy relation
MoveModel.IR RefElim     ~=       MoveModel.IR reference semantics
```

The first relation is source compiler correctness: the source relation may be
more abstract, but it must contain every compiled behavior. The second is the
existing reference-elimination correctness direction. Once the first relation
is proved, together they will allow a source theorem to justify the generated
IR and let IR-level verification remain the common boundary for mixed modules.
XIR then needs a representation-preservation argument before compiler v2; no
new functional verification semantics belongs at the XIR layer.

The source prophecy rules should deliberately reuse the terminology and
algebra of `MoveModel.IR.RefElim`:

- immutable-reference erasure;
- checked-out mutation values;
- mutation paths;
- deferred write-back;
- liveness-defined loan death;
- dynamic parent selection at control-flow joins.

## Imported modules

Lean-authored dependencies already use ordinary Lean imports. Their actual
definitions and associated `sourceSpec`, contract, and proof declarations are
available to the caller's proof, while executable compilation records a
qualified external call and orders both XIR modules by dependency.

Importing a module authored in Move into a source-level Lean proof is the next
stage. Such dependencies enter through `MoveModel.IR` and already have an
executable and verification semantics. A source call to one will use one of:

1. an IR-derived verified summary;
2. an explicit assumed interface, clearly marked as an assumption;
3. inlining into the shared IR proof, where feasible.

Summaries express mutable arguments as pre/post value relations. This is the
same interface used for Lean-authored callees, so prophecy details do not leak
across the language boundary.

Runtime type tags and generic functions remain explicit. Source proofs either
reason parametrically or use the existing finite monomorphization verification
view with its tag-collision obligations.

## Current module layout

```text
Move/
  Semantics/
    Outcome.lean          success, abort, and rollback
    Spec.lean             relational verification semantics
    Global.lean           typed global resources
    Checked.lean          bounded integer operations
    Reference.lean        prophecy and ownership-passing borrows
    Vector.lean           checked vector operations and element loans
  Verify/
    SimpAttrs.lean        shared simp-set registrations
    Contract.lean         contracts, satisfaction, and the wp core
    WP.lean               one-obligation wp rules for primitives
    Borrow.lean           prophecy and reborrow proof rules
    Compare.lean          structural Move comparisons
    Syntax.lean           `spec`, source-semantics generation, and `verify`
    Tactics.lean          proof tactics and the simp-set inventory
```

The intended compiler-correctness proof does not yet have checked-in modules:

```text
  Compiler/
    Correctness/
      Types.lean
      Expressions.lean
      References.lean
      Functions.lean
```

That split may evolve. Independent semantic definitions and proof helpers
should remain in separate files rather than accumulating in one translation
theorem.

## Implementation stages

### 1. Faithful outcomes and arithmetic — implemented

- Define typed global state and explicit relational normal/abort behavior.
- Define bounded `U64` values and checked operations.
- Keep the surface notation and preserve current compiler recognition.
- Verify a pure and a resource-mutating arithmetic example directly in Lean.

### 2. Reference core — implemented

- Define immutable-reference erasure.
- Define `MutRef.current` plus ghost prophecy and the relational loan rule.
- Add local read/write/freeze tests and lemmas.

### 3. Structured reborrows — implemented for the current source subset

- Add typed field lenses and checked vector-element lenses.
- Implement nested prophecy composition.
- Prove restoration for fields, vector elements, and multiple disjoint
  siblings.
- Test normal, aborting, and out-of-bounds paths.

### 4. Global resources — implemented for borrow/read/write

- Implement scoped checkout and restoration in the source semantics.
- Prove rollback behavior with a live mutable loan.
- Verify the account deposit and withdrawal examples directly.

### 5. Borrow certificates and automation — implemented for retained source

- A generic retained-source control topology is shared with the poison-aware
  borrow-effect projection.
- A small `WellBorrowed` checker replays access-path transfers, loop facts,
  call effects, separations, and returned-reference derivations.
- Each generated source specification exports `borrowProgram`,
  `borrowCertificate`, and `wellBorrowed` declarations before proof
  generation.
- Compiler-correctness transport of this source certificate to `MoveModel.IR`
  remains phase 7, not part of the source proof's trusted premise.

### 6. Calls and loops — partial

- Add prophecy-passing summaries for `&mut` parameters and results.
- Support Lean-authored calls through generated summaries; imported-Move
  summaries remain future work.
- Generate direct-call semantics by composing the callee's generated
  `sourceSpec`.
- Interpret direct self-recursion by existential finite unfolding and expose
  fixed-point induction through `Move.Verify.satisfies_fix`. Ordinary calls
  and explicit `continue` calls share this semantics.
- Mutually recursive strongly connected components use one heterogeneous
  `Spec.fixFamily`; `contract_intro` exposes the family-wide induction step.
  General loop-invariant inference remains future work.

### 7. Compiler correctness — not implemented

- Prove type/value correspondence from source types to LIR and IR types.
- Prove expression and checked-operation lowering.
- Prove mutable-loan lowering against IR reference semantics.
- Compose with reference-elimination correctness.
- State and prove module-level preservation sufficient to transport source
  contracts to emitted XIR and verified bytecode assumptions.

## Test strategy

Each semantic feature needs Lean proofs plus compiled execution comparisons:

- checked arithmetic success and each abort class;
- immutable local, field, vector, and global borrows;
- mutable local and global updates;
- nested field and vector-element updates;
- freeze after write;
- disjoint sibling borrows;
- rejected overlapping aliases and use of suspended owners;
- normal and aborting function calls with `&mut` arguments;
- a live global loan followed by abort and transaction rollback;
- branches where loans die on different edges;
- tail-recursive loops carrying mutable values;
- generic resources whose runtime type tags are equal and unequal.

For accepted programs, tests should compare:

1. the source prophecy relation and proved contract;
2. `MoveModel.IR` interpreter execution;
3. transactional compiler-v2/VM execution where supported.

Rejected-program tests should check source-positioned diagnostics from the
borrow validator rather than failures deep in IR lowering.

## Proof obligations

The implementation is incomplete until the following properties are proved:

- immutable-reference erasure preserves observable reads;
- nested prophecy reconciliation reconstructs the owner value correctly;
- global checkout/restoration preserves unrelated resources;
- abort hides all tentative write-back;
- borrow certificates imply the assumptions of the source reference rules;
- source checked operations agree with IR arithmetic and abort codes;
- source calls agree with their summaries and compiled callees;
- accepted source control flow lowers to behaviorally equivalent LIR/IR;
- source contracts transfer across compilation.

## Settled decisions

- `Action` contains transaction effects, not reference storage.
- `&T` is represented as a dereferenced read-only value in verification.
- `&mut T` is represented by current and prophesied-final values.
- Prophecies are ghost variables constrained at loan death.
- Lean source semantics is relational and used only for verification.
- Execution is defined solely by compilation to IR and bytecode.
- Loan scopes are generated from retained source sequencing, not user
  annotations, and source proof generation is gated by a checked borrow
  certificate.
- Prophecies and reference identities are absent from user-facing contracts.
- The source proof layer complements, rather than replaces, IR semantics and
  bytecode verification.

## Open design questions

- How to extend automatic source semantics to the constructs still rejected
  at `spec`: the global publication/removal primitives, pure Move callees,
  and effectful callees with mutable-reference parameters.
- The surface syntax and induction support for loop invariants and mutually
  recursive SCCs.
- How much `verify` should elaborate into generated simplification versus a
  tactic over explicit WP terms.
- Which reference-returning function signatures are admitted in the first
  source-verification milestone.
- How an imported Move module exposes a proved or explicitly assumed summary
  to source-level Lean verification.
- Which preservation theorem and certificate boundary best insulate the
  source-to-IR proof from Lean compiler LCNF changes.
