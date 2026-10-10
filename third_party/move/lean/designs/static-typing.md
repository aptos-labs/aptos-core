# Static typing and preservation

Status: Phases 1–4 done (2026-10-01); Phase 5 in progress (loan independence done 2026-10-01). Decided: preservation of runtime
typing for the big-step semantics (`higher-order-functions.md`, "H4:
decisions", 1), built first as a proved-sound static type checker that the
agreement proof (`denotation.md`) reuses.

## Why

An invocation of a closure the proof cannot see (`invoke` of a function
value from an input) denotes `closureMeaning`, whose `undefined` outcome is
a run of the target returning results that do not decode at the closure's
type. Only preservation excludes it: a well-typed function run from a typed
state on typed arguments returns typed results. The agreement theorem
(`compileFunction_agrees`, assumed) needs the same facts, as a strengthening:
its invariant relates runtime frames to the denotation's typed
environments.

## What validation provides today

Nothing a proof can use about types:

- The typing rules are `private partial def`s returning diagnostics
  (`Validation/Capability.lean`, `scanExpr` and its block). They cannot be
  unfolded, and several skip silently when information is missing.
- `prepareExecution` switches the typing checks off (`typingGate
  .execution _ = #[]`), and `ValidatedUnit` can be built by any importer
  (`Internal.mkValidatedUnit`); `prepareExecution … = .ok executable`
  yields only the unit indexing `executable`'s type and two equations.
- An elided generic instantiation is solved for checking and not stored
  (`inferredInstantiationDiagnostics`): the call runs with an empty
  instantiation (`callTypeInstantiation` returns `#[]`), so the callee's
  frame keeps its symbolic types.
- Instantiation searches the interned arena (`instantiatePlaceFieldType?`)
  and falls back to the symbolic type when the node is absent
  (`instantiatedTypeId`); nothing guarantees presence, so a global key can
  name a symbolic type.
- No invariant states that every namespace shares the unit's tables or that
  a namespace's identity is its index.
- Borrow and initialization certificates are receipts; liveness of
  references after their loans die is not stated.

## Phases

1. **Static typing (this document's subject).** Semantic types; a
   declarative typing relation over validated LIR; a total checker proved
   sound against it, run by `prepareExecution`, whose result carries the
   proof; inferred instantiations stored; instantiation closure, arena
   uniqueness, and table/identity checks. Reviewed before Phase 2.
2. **Defensive semantics (done 2026-10-01).** Of the cases the borrow
   checker excludes, two were ill-typed rather than merely odd, and now are
   not: a dereference at a mutable reference type requires a live borrow
   (`dereferenceBorrow?`; a shared operand is the value itself, by its
   site), and a global write-back whose slot no longer holds the loan's hole
   is exported as pending instead of overwriting the slot
   (`fillVisibleHole`). The others stay and are typed in Phase 3: a hole is
   typed at its loan's referent wherever it is (a borrowed global slot, a
   read of a borrowed local, a moved value); the `.unit` a settled borrow
   leaves is a dead mutable reference, which (a) makes useless; a taken
   slot's hole is typed, and its write-back now goes to pending. Rollback
   resets `nextLoan`, so the loan environment ranges over loans below it.
3. **Operation lemmas (done 2026-10-01).** Each runtime operation keeps
   frames and states typed and produces values of the type the checker
   assigns; see "Phase 3".
4. **Induction (done 2026-10-01).** Preservation over the six big-step
   relations, natives as a hypothesis (`nativeCall` is opaque); see
   "Phase 4".
5. **Use.** The closer's rule for invoking an unknown closure (H4c), and
   the typing half of the agreement proof.

## Phase 1

### Semantic types (done)

`Validation/SemanticTypes.lean`. `SemTy` is a type as values inhabit it:
names spelled out, type parameters replaced by semantic arguments
(`SemArg`), lifetimes, abilities, and a reference's profile erased.
`SemTy.resolveFuel tables env fuel typeId` reads a table entry under an
environment of semantic generic arguments; `Resolves` is resolution at
some fuel, deterministic (`Resolves.unique`), characterized node by node
(`Resolves.node`). `Resolves.instantiate` relates the runtime's interned
instantiation to resolution under the arguments: where
`instantiatePlaceFieldTypeFuel?` finds the instantiated node, it resolves
without parameters to what the declaration's node resolves to under the
arguments.

A generic body is checked once, under the environment binding each type
binder to its rigid parameter (`SemTy.param`); `Resolves.subst` carries the
result to any instantiation. The module sits below `ExecutableUnit`, with
the runtime's instantiation primitives (`Validation/Instantiate.lean`), so
that preparation can run the checker.

Validation now stores the instantiation it solves for an elided generic call
or closure (`inferInstantiations`).

### Values

`Semantics/ValueTyping.lean`. `HasType unit loans value type` types a
runtime value at a semantic type, a live loan's borrow and hole at the
loan's referent (`LoanTypes`). Phase 3 adds a dead mutable reference
(`.unit` left where a settled borrow rested) once Phase 2 makes every use
of one stuck.

### Declarative typing

Per function, under its generic environment: every expression reachable
from the body's root satisfies its node rule, stated over the stored
`typeId`s resolved to semantic types. A rule states what preservation reads
of the node, not every check validation makes. Examples:

- `localVar l`: the local's declared type and the node's type resolve to
  the same semantic type.
- direct call: the target resolves; the call's instantiation has the
  target's generic arity and resolves to semantic arguments `A`; each
  argument's type is the target's parameter type under `A` (in the target's
  table); the node's type is the target's packed results under `A`.
- closure: as a call for the captured parameters; the node's type is the
  function type of the open parameters and the packed results under `A`.
- `invoke`: the callee operand's type is a function type whose parameters
  are the remaining operands' types and whose result is the node's type.
- global operation: one type argument, the resource; its type under the
  frame's environment is the key's type; take, borrow, and publish relate
  the value or referent to it.
- primitives: `WfPrimitive` (`Semantics/Typing.lean`), restated over
  semantic types.
- places and patterns: a place's type is its root's type projected;
  a pattern's bindings take the types of the matched parts.

The relation is a `Prop` per function (`FunctionTyped`) and per unit
(`UnitTyped`: every namespace's functions with structured bodies).

### The checker (done)

`Validation/StaticTyping.lean`. Each node's rule is a total `Bool` check
over semantic types (`Context.checkNode`); a body is walked from its root
(`checkTree`), and `checkNode_of_reaches` states that every node a checked
body runs, with the context it runs in (`Reaches`: a loop's body inside its
loop), satisfies its check. `checkUnit unit pointerWidth` checks every
function and constant and that each namespace sits at its index and reads
the unit's types and names. `prepareExecution` runs it, and the
`ExecutableUnit` it returns carries `typed : checkUnit unit
targetPointerWidth = true`: a theorem quantifying over a prepared unit gets
the facts without kernel evaluation. A refused unit is not prepared
(`LIR-STATIC-TYPE`, naming each failing node with its type and its
operands'), and verification refuses to state theorems about it, which
would be vacuous (`ensureUnitDefinition`).

Rules worth noting:

- An expression stops — it diverges (`diverts`: control transfer, a part
  evaluated first stopping, a loop no `break` targets) or has type `never`
  — and then goes anywhere.
- A target the unit does not hold (a reference into a namespace linked
  elsewhere) has no evaluation, since the runtime resolves it by the same
  lookups; the node is accepted.
- Pointer-sized integers are read at the target's width (`targetWidth`),
  which a unit without one does not have.
- A field place on an enum without a downcast has the type every variant
  holding the field agrees on; a tuple is indexed by a literal position; a
  subslice of a vector of static length has a static length.
- Data operations select through a reference of either kind.

Tests: `LeanerIR/Tests/StaticTyping.lean`. Every Check fixture's units,
the suites' units, and the Rust exporter's are accepted.

### Instantiation (done)

- Validation stores a solved elided instantiation on the call or closure it
  was solved for (`inferInstantiations`), so a generic target always runs
  under its arguments.
- A frame reads its instantiation at the types it *consults*: the resource
  types of its global operations and the type arguments of its generic
  calls and closures. The runtime finds an instantiated type by searching
  the arena for its node (`instantiatePlaceFieldType?`), and
  `Resolves.instantiate` makes a found node exactly the type under the
  arguments; what must be ensured is that the search finds one.
- Closure (`checkInstantiations`): a callee's required types, instantiated
  at a call's (or closure's) arguments, must be interned, mention no lifetime
  parameter, and are required of the caller. The table is iterated to its
  fixpoint (`requiredTypes`, bounded by the arena; polymorphic recursion has
  none) and checked by a separate pass (`requiredClosed`). No uniqueness of
  the arena's nodes is needed: the callee's search for a required type
  repeats, node by node, the caller's search for its instantiation, so the
  two find the same node. A lifetime parameter would break that repetition
  (the callee's search keeps the call's lifetime, the caller's maps it),
  hence its exclusion.
- A source writes the instantiations it names, not those a callee consults
  through a further generic call. A registered unit is closed under them
  (`ValidatedUnit.internRequiredTypes`): each instantiation the arena lacks
  is appended, node by node, as the search locates it, so the check then
  finds it. Appended types are new identifiers; nothing that names an
  existing one changes.

### Tables and identities

The checker requires every namespace to carry the unit's tables and its
index as its identity; nominal types then compare by `QualifiedName`
across namespaces.

### Tests

Assertion tests in `LeanerIR/Tests/` for the semantic-type lemmas'
examples, the checker on accepted and rejected units (each rule's failure
with its diagnostic), and the four suites unchanged except for units the
new checks reject, each analysed (a frontend omitting an instantiation is
fixed in the frontend, not admitted).

## Phase 3

`Semantics/StateTyping.lean` and `Semantics/LoanTyping.lean`. A frame is
typed when each local's value has its declared type under the frame's
environment (`TypedFrame`); a state when every global slot has its key's
type, every pending write-back its loan's referent, and every loan is below
the frontier (`TypedState`). Every lemma is stated for one loan environment;
only borrowing extends it (`TypedState.mint`).

- Primitives, constants, nominal construction and destruction, and the data
  operations (field selection directly, through a reference, and across
  variants; variant tests; discriminants; field updates); result packing.
- Places: a place the checker types resolves to a path from a local that
  reads and writes at the place's type (`PathTyped`, `PlaceTyped`,
  `resolvePlace_typed`).
- Loans: every loan operation rewrites one node — a hole filled, a borrow
  retired to the unit, a borrow's current replaced — and keeps every type
  the value has (`Subsumes`), whatever position the node holds; so
  write-backs, settling, frame export, pending application, mutation, and
  returned reborrows need no place typing. Borrowing mints the frontier loan
  at the place's type.
- Global operations, initial frames, and pattern binding.

The checker gained four facts the lemmas need, none of which a validated
unit violates: an unresolved struct target is accepted only where the
runtime's own lookup fails (`declarationTarget?`); variant names are
distinct; every declared discriminant fits the node's type; and a
dereference's borrow certificate agrees with the reference's kind (the
runtime reads a certified shared dereference as its base). A hole is typed
at its loan's referent up to outer shared references, the value a shared
reference observes.

Left to Phase 4: the transport from the checker's static environment to a
frame's (each node rule is preserved by substitution), the links between
runtime and checker lookups at calls and closures, and `stops` ⇒ no value.

## Phase 4

The induction runs at the *runtime* context of a frame: the static context
under the frame's semantic arguments (`Context.subst`). Three pieces make
that context checked and its frames faithful.

- **Transport** (`Validation/StaticTypingSubst.lean`, done). Every rule is a
  positive check of equalities and shapes of types, so a body checked under
  its rigid parameters checks under any arguments (`checkTree_subst`). The
  one rule that branched on a type's shape with an accepting fallback —
  selection through a reference — now accepts either reading, since a
  shared reference is the value it observes.
- **Faithful frames** (`Semantics/FrameInstantiation.lean`, done). A frame
  reads its instantiation faithfully (`FrameInstantiation`) when it has none
  and every type it requires is closed, or when its instantiation is the
  table of closed type arguments that instantiate each such type. A call
  from a faithful frame creates one (`FrameInstantiation.call`): the
  callee's arena search at the caller-rewritten arguments repeats, node by
  node, the caller's search for the instance of the callee's type at the
  call's own arguments (`instantiate_repeat`). The checker states the needed
  closure at each call (`Context.edgeClosed`: the target's required types
  instantiate to types the caller requires, free of lifetime parameters)
  and requires generic argument kinds to match their binders; a closure
  value carries the invariant.
- **Runtime decisions read off types.** Where the runtime decides by a
  validation record or an arena node rather than by a value — a certified
  shared dereference of a place, a shared operand of a dereference or
  freeze — the checker states that the record agrees with the type, so the
  facts hold at the runtime context too.

- **Induction** (`Semantics/Preservation.lean`, done). `preservation`: a
  function run on arguments of its parameter types, from a state typed
  under some loans and at a faithful frame, ends in a state typed under
  loans extending them, and a return carries values of its result types.
  Each expression relation is proved for every placed, checked context: run
  from a frame whose locals are typed and whose instantiation it reads
  faithfully (`Running`), it ends in such a frame, with its control typed as
  the checker types the node — a value of the node's type from a node that
  does not divert, a break at its loop's type, a return at the function's
  results. Natives are a hypothesis (`NativesTyped`). The theorem depends on
  no axioms beyond Lean's standard three.

The induction needed two more facts: a hole's referent is inhabited, so no
value has type `never` and a node of that type yields none; and an
executable unit's target width is its profiles' (`ExecutableUnit.width_eq`).

## Phase 5 (in progress)

The closer's rule for invoking a closure a proof cannot see. `Satisfies`
requires a computation to have no undefined outcome, and an invocation is
undefined where a run of the closure's target returns results that do not
decode at the invocation's type (`closureMeaning`). Preservation excludes
that, given three facts about the run (decided 2026-10-01):

- **Inputs and initial state.** The closure is typed (`ClosureTyped`: its
  target, frame, and captures as the closure rule states them, with the
  target's result row exactly the invocation's), its arguments are typed,
  and global memory is typed (`GlobalsTyped`). These are hypotheses of the
  theorems that need them — a function whose denotation invokes a closure it
  cannot see, directly or through callees that need them — and its callers
  discharge them at the call. Other functions are unchanged. Natives are a
  hypothesis as before (`NativesTyped`).
- **Pending write-backs.** A start state's pending write-backs are
  arbitrary (`Admissible`), and a run never reads them: it only appends, and
  applies only what its callees append. `TypedState` leaves the write-backs
  pending at a run's start untyped (`inert`).
- **No holes in results.** A typed value may be a loan hole, which reading a
  borrowed place yields and only the borrow checker excludes. A function
  returns only results that hold no hole outside a borrow
  (`Outcome.holeFree`); otherwise its run is stuck. Validated code behaves as
  before.

A typed result without holes decodes at the invocation's type
(`RowDecodes`, by the type's structure); the rule then reads the invocation
through the behavioral predicates (done: `Proofs/Invocation.lean`).

### Loan independence (done)

A behavioral predicate runs the invocation from any start with the given
global memory (`StartsAt`), as a call's meaning does, and starts differ in
their loan bookkeeping: the loan frontier, the pending write-backs, and the
global loan registry. A successful run must refute `aborts_of` (a contract's
`aborts_if` is sufficient), and `result_of` must be the result of every run;
both need runs from such starts to agree, as the agreement proof does.

Two runs from starts with the same global memory holding no loan, on
arguments holding none, mint the same loans in the same order, offset by
their frontiers' difference; neither reads the write-backs pending at its
start, and each registers only the loans it mints, which the registry's
freshness keeps apart from what it holds. So the second run is the first
with its loan identities renamed, and outcomes holding no loan are equal.
Where global memory holds a hole of a loan at or beyond a frontier, the
claim fails: a minted loan collides with it.

The proof (`Semantics/LoanIndependence.lean`) shifts loan identities by an
offset on values, frames, and global memory (`Semantics/LoanRenaming.lean`)
and relates a state to the second run's (`StateShifted`): global memory
shifted, the frontier raised by the offset, the write-backs appended after
each run's start shifted, and the registry's entries minted by the run
shifted before entries below each start's frontier. `Above frontier` states
that every loan a value or frame holds was minted at or beyond the frontier;
a value is above a frontier exactly when it is a shift by it, so each
operation lemma is one commutation (`LoanRenamingOps`, `…Primitives`,
`…Effects`, `…Functions`). The value order and structural equality read
loan identities, and a shift preserves both. A forward induction over the
first run's derivation builds the second (`shifts`); natives are a
hypothesis (`NativesShift`), as for preservation. `runs_mirror` instantiates
it at two starts; with determinism, `evalFunction_agree` relates any two
runs, a returning run refutes `aborts_of` (`not_abortsOf_of_ok`), and its
results, holding no loan, are `result_of` (`resultOf_eq_of_ok`).

### Integration (in progress)

The rule (`wp_closureMeaning_unseen`, `Proofs/Invocation.lean`): an
invocation of a typed closure a proof cannot see, on typed arguments from
typed global memory, has the weakest precondition its predicates state —
a run's results are those `ensures_of` names, refute `aborts_of`, and are
`result_of`; an abort is one `aborts_of` names. Its premises reach a
theorem as follows, in this order:

1. **Assumptions.** A contract has a generated `assumes` field, `True`
   unless a function needs it, which `Satisfies` takes before `requires`
   and every call discharges (`wp_call`); `requires_of` still reads only
   `requires` (done).
2. **Types of rows.** A native type corresponds to a semantic type, so a
   native argument's encoding is typed (`RowTyped`) and a typed, hole-free
   result decodes (`RowDecodes`). Done for scalars (`NTy.scalarType?`) and
   for type parameters, whose semantic types depend on the frame: a
   function value's typing states them (step 3); tuples, vectors, and
   nominal types remain.
3. **Statements.** A verified function with a parameter of function type,
   or calling one through verified bodies, assumes its initial global
   memory `GlobalsTyped`; its theorem takes `NativesTyped` and
   `NativesShift` (done, `Contract.assumesTyping`). A function value is
   typed by its carrier where the unit's readings agree
   (`ClosureTypedAt.ofCarrier`, C3c in
   [`higher-order-functions.md`](higher-order-functions.md)): at rows the
   frame resolves to scalars, `ClosureTypedAt` states the semantic types the
   target's typing gives, at which the row's values are typed and typed
   results decode at the frame. A generic function whose function-typed
   parameter's rows mention its type parameters assumes that its frame
   resolves them to scalars (`ScalarAt`).
4. **The closer step.** At `wp (closureMeaning …)` on a closure not built
   in the body, `wp_closureMeaning_unseen_at` with the assumptions; a
   run's `result_of` reads are rewritten by its results. Done for
   invocations from the initial state. An invocation in a callee inlined at
   type arguments is the invocation at the caller's types
   (`closureMeaning_instantiate`).
5. **Callers.** A caller discharges a generic callee's `ScalarAt` at the
   rows the callee's frame resolves (`NRow.resolved`: an induced frame
   substitutes its type arguments, and a type without parameters resolves
   to itself), by evaluation or by its own assumption, and global memory at
   a call from the caller's initial state by the caller's own assumption
   (done). A `result_of` read of a literal closure of a non-generic target
   is typed by its target's rows computed over the unit (`closureTypes?`,
   `ClosureTypedAt.ofClosureOf`). Memory stays typed through the function's own global
   operations: a stored value's typing is decided over the unit
   (`GlobalsTyped.insertSlot`, `leaner_has_type`), and a removal keeps it
   (done for scalars and structures of them). After a call, memory is the
   callee's exit, which preservation types but which may hold a loan's
   hole; a callee with a loan-free interface leaving none needs either a
   defensive exit check, as for results, or a proof that a call settles the
   loans it mints (open).
6. **Frames (H4d).** Without `modifies_of`, a function parameter leaves
   global memory alone: a generated precondition of the higher-order
   function, discharged from the target's contract frame.

### Not in Phase 1

Semantics changes, the loan invariant, operation lemmas beyond the
existing ones, the induction, and the closer rule.
