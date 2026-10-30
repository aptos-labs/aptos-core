# One denotation, one agreement proof

The verification design: one typed denotation of validated VIR into
`Spec`, and one agreement theorem against the big-step semantics. It
replaces the per-target generated agreement proofs of
[`certifying-execution.md`](../historical/certifying-execution.md),
[`generic-route.md`](../historical/generic-route.md) and
[`verification-v2.md`](../historical/verification-v2.md). Open work is in
[`roadmap.md`](../plan/roadmap.md); the hypotheses no proof discharges are
in [`assumptions.md`](../status/assumptions.md).

## What does not change

- **`BigStep` is the authority.** A `verify f` theorem is a theorem about
  the big-step relation of validated VIR, connected by proof, never by
  assumption. The elaborated VerexSL term is produced by frontends and a
  metaprogram, so a theorem only about the elaborator's output would have
  an unfalsifiable trust root.
- **One semantics.** The interpreter, the big-step relation and the
  verifier run one model, including the prophecy encoding of references
  ([`prophetic-references.md`](prophetic-references.md)). The interpreter
  is the executable form and the MonoVM differential anchor.
- **Vacuity.** A bare `verify` success is meaningful only over VIR every
  node of which has executable semantics. Preparation rejects the rest.
- **The three claims** in [`CLAUDE.md`](../../CLAUDE.md) stay separate:
  source verification, lowering to bytecode, and the unproved
  compiler-correctness theorem between them.

## The design

Two definitions and one theorem, written once:

```
denote : ExecutableUnit unit → FunctionHandle →
  Array RuntimeValue → Spec RuntimeState Failure (Array RuntimeValue)

denote_agrees : ∀ unit function arguments,
  Spec.Equiv (denote unit function arguments) (meaning unit function arguments)
```

- `denote` is a Lean function of the validated unit itself, recursing over
  the body's expression arena by structural recursion on a fuel bounded by
  the arena size, never by well-founded recursion, so that the kernel and
  `simp` unfold it on concrete data. Every VIR construct is one case.
  Calls consume the callee's meaning; recursive cycles are least fixed
  points; loops are fixed points under their invariant; storage and
  references use the typed memory and the prophecy encoding. `denote` is a
  relation, not an executable.
- `denote_agrees` is proved once, by induction against
  `BigStep.EvalFunction`, one case per construct. It is what makes a bare
  `verify` meaningful: vacuity can arise only from preparation admitting a
  node without executable semantics.
- Per target, `verify f` states the contract over `denote unit f`. The
  elaborator unfolds `denote` on the concrete unit to a closed shallow
  term `t`, certifies `denote unit f = t` definitionally, and reasons
  about `t`. Nothing is proved per target about agreement.
- `denote` is typed and frame-free: indexed by the validated type, over
  native carriers, locals bound by continuations, references as prophecy
  values. No `RuntimeFrame`, row, or loan registry occurs in the unfolded
  term; the codecs appear once, in the agreement induction, never in a
  goal.
- The solver is general: a `Spec.wp` rule per combinator in a simp
  inventory, then `omega`/`decide`/`grind` on leaves.

Adding a construct costs one `denote` case, one case of the induction, and
at most one wp lemma. Unfolding `denote` is a translation over concrete
data, done once per target before any obligation exists; the cost gate
measures it separately from closing.

### Modules

- `VerexIR/Proofs/Denote/`: `Types` (native carriers, rows, codecs, the
  state operations and their wp rules), `Term` (typed terms and
  `Term.denote`), `Compile` (`compileFunction`, structural on fuel),
  `Agreement` (the named axioms `compileFunction_agrees` and
  `compileFunction_least_cycle`, and the transport to
  `SatisfiesFunction`), `Solve` (the solver, the `vir_denote` and
  `vir_denote_norm` simp sets, the leaf tactics), `BitLift` (a leaf
  restated over bit vectors under `pragma bv`).
- `VerexSL/Verify.lean` owns `verify`, which audits each target's
  artifacts (the denotation route, approved axioms only) and publishes per
  target the quoted term, the kernel `rfl` certificate of its compilation,
  `typedVerified` over the denotation and `verified` over the big-step
  meaning. `Contract.lean` keeps the clause translator and preparation.
- `VerexSL/Tests/DenotePerformance.{lean,exp}` gates heartbeats and proof
  objects per target; the e2e check driver caps a target at 180k
  verification heartbeats.

## Principles

Each principle is the inverse of a measured cause of cost in the earlier
routes ([`verification-perf-audit.md`](../historical/verification-perf-audit.md)).

1. **The goal contains the mathematics, not the machinery.** The term a
   `verify` reasons over mentions only what the source mentions: values of
   native carriers, Lean binders for locals, prophecy values for `&mut`,
   the typed memory for globals ([`static-memory.md`](static-memory.md)).
   The elaborator audits the constants of the unfolded term and rejects a
   target mentioning a frame, row, registry, arena, id or string.
2. **Translate once, before any obligation; never symbolically execute.**
   All computation over program data happens in the unfold of `denote`,
   before the first obligation exists. An obligation never contains
   `denote`, an evaluator, or an arena lookup.
3. **Weakest preconditions, never unfolded relations.** One `wp` rule per
   combinator, `wp (bind a f) ↔ wp a (fun v => wp (f v))`: no existential,
   each sub-term once, linear in the body. Well-definedness is structural.
4. **One traversal, one context.** The body is traversed once, split by
   exit path. Context-wide passes run only on leaf goals over the leaf's
   own context.
5. **Modularity where declared.** An `opaque` callee contributes its
   contract at the boundary, never its body; a generic body is proved once
   and instantiated; every other call is inlined, as the Move Prover does.
6. **Decidable leaves, structural everything else.** Leaves are linear
   arithmetic over `Int` with range certificates, or finite enumerations.
   No tactic searches over obligation structure; the solver is the simp
   inventory followed by `omega`/`decide`/`grind`, with no route selection.
7. **Numeric identity in hot paths.** Ids, storage keys and variant
   indices are `Nat`; every literal has one spelling.
8. **Precomputed inventories, stable keys.** Every simp inventory is a
   registered simp set, never an explicit list (`simp only [list]`
   re-elaborates every entry per call). The types that key rewrite lemmas
   (`HList`, `variantCarrier`) are not reducible: a reducible key is
   reduced in goals but stuck in lemmas.
9. **Measured per target.** The cost gate records heartbeats, proof
   objects and unfold cost per target; a regeneration names only the
   targets a change was meant to move.

Principles 1, 2, 3 and 6 are architectural: they hold or fail by
construction of `denote` and the solver. The others are engineering
discipline.

## What the denotation carries

Scalars and checked arithmetic, comparisons, Boolean operations, unsigned
`&`, checked shifts and casts, constants, `if`, `let`, blocks, `abort`,
`assert`, early `return`, assignment, `break`/`continue` (labeled too),
loops with `invariant` clauses, direct calls, the Rust profile's modular
arithmetic (`Term.modular`), tuples, structs, enums with variant tests and
payload selection (a field of another variant makes the profile's mismatch
throw), matches and destructuring bindings (`Term.caseOf`: typed patterns,
guards, the mismatch throw where no arm is taken), shared borrows of
locals, mutable references (`NTy.ref`), storage over the keyed global
memory (`globalRead`, `globalContains`, `globalBorrow`, `globalPublish`,
`globalTake`), vectors (`NTy.vector`, the bounded `SpecVector`: element
places, literals, `index` with its abort, the bounds check the lowering
emits, `push`, `insert`, `remove`, `swap`, `concat`, `slice`,
`reverseSlice`, `contains`, `indexOf`, `destroyEmpty`; a construction
exceeding the bound is undefined). A rejection names the construct.

**Loops.** A loop carries a state invariant beside the local frame: the
store and registry when the body has no global effect, the pending set when
it has no call, and the loan discipline from the entry state. An automatic
frame ties every immutable local available at the header to its entry
value; the invariant asserts every local it reads defined. `old(p)` in a
loop invariant reads the parameter at the function's start (recorded as a
`FunctionStart` hypothesis per function; an inlined callee's start is
recorded at its call); a local under `old` or inside a state anchor is read
at the loop's entry. The data invariants of a struct-typed local the body
modifies are conjoined to the loop's invariant, as the Prover assumes
well-formedness of a havocked value. A mutable loan the body declares dies
at a `continue` of its own loop. Loop sites pair the namespace with the
expression, so loops of callees from several namespaces stay distinct. At
the iteration the whole context is normalized and its equations
substituted once.

**Calls.** A callee is inlined, specified or not, and the caller reasons
over its compiled body through the agreement theorem
(`wp_calleeMeaning_of_compiled`). Only a native, an `opaque` callee and a
member of a cycle of calls are used through their contracts
(`usedThroughContract`); such a callee with a body is used by its theorem,
verified first. A callee without a theorem, whose verification failed or
did not run, is assumed by its callers, its failure reported where it is
verified, as the Prover verifies each function against its callees'
specifications; a callee whose verification is in progress is refused
(`inProgressTargets`), as assuming it would be circular. A contract has two
views: a body is proved against its unmarked and `[concrete]` clauses,
callers see its unmarked and `[abstract]` ones and assume the abstract
view as a hypothesis of their theorems. Natives run as the runtime
implements them (`EvalFunction.native` and the interpreter share one fixed
but unknown `BigStep.nativeCall`); a caller's theorem assumes each native
it reaches satisfies its contract, never as an axiom. A native returning
`&mut` is carried when its contract states the returned reference's
final value; otherwise the call is rejected.

**Equality.** The codecs are tight (`NTy.encode_eq_iff`), so an encoding
equation is a decoding equation in both directions, and `eqb` decides
equality on reference-free types. `compare` (`std::cmp::compare`) is the
structural order: primitives by their natural order, vectors, tuples and
fields lexicographically, enum values by variant position; proved a total
order (`VerexIR/Proofs/Order.lean`).

**Bit vectors.** Under `pragma bv` a leaf the integer deciders leave is
restated over bit vectors (`BitLift`): each certified unsigned value
becomes a vector of the leaf's widest width, bit operations, shifts and
remainders become vector operations, a part that does not lift becomes a
Boolean atom, and `bv_decide` decides the result.

## Specification functions, lemmas, modules

- A recursive `spec fun` names a `decreases` measure (a mathematical
  integer, its contract's `decreases` condition) and is a Lean definition
  `f.spec` over its bundled arguments by well-founded recursion on it;
  each recursive call's decrease is proved by `omega` from the conditions
  on its path. A recursive call is never expanded in a contract. A leaf
  instantiates `f.spec.unfold` once at each application it holds and keeps
  the instances whose guards the context decides (`verex_unfold_specs`);
  `grind` relates the applications by congruence. A proof needing
  induction is authored. A Boolean result is a proposition.
- Mutually recursive `spec fun`s (a strongly connected component) are one
  well-founded fixpoint over the sum of their bundles, each member the
  fixpoint at its injection, each member's measure chosen among the
  members' candidates.
- A recursive `spec fun` or a lemma reading storage takes the memory and,
  before it, the family (`Skolems`) its reads resolve resource types at; an
  application passes its own frame.
- A lemma's parameter, and a quantifier's binder over a type, ranges over
  the native values of the type, as the Prover assumes.
- A specification function applied in a clause is expanded at its
  arguments' types; an abort or an opaque specification function denotes
  a fixed unknown value (`Contract.opaqueSpec`, keyed by the qualified name
  and the native type arguments under the contract's family).
- A module elaborates in three passes, so nothing among its items needs
  to precede its use: Verex items registered and recursive specification
  functions defined; `theorem` items elaborated in source order in the
  module's namespace; verification targets, callees first. A function
  with a body and a `spec` block is verified automatically unless its
  specification or its module sets `pragma verify = false`; a `verify f`
  item states a target explicitly, with `by` an authored proof. Theorems
  are the only Lean items: the module is parsed as one command, so a
  tactic macro among its items could not be used by them. A top-level
  `verify path` remains for a `#guard_msgs` rejection and for a module
  declared in another file.
- `verify f by lean => tactics` proves `f`'s goal by the tactics. `verex_obligations`
  runs the walk without deciding a leaf: every leaf stays a goal tagged by
  what it owes (`ensures_2`, `aborts_if_1`, `invariant_1_step`,
  `modifies_1`, `requires_1_of_g_at_call_2`), suffixed `_1`, `_2`, … where several
  leaves owe one clause. The automatic verification is the same walk with
  every leaf decided as it is reached (`verex_solve_goals`). `verex_prepare`
  normalizes a leaf once, `verex_decide` runs the bounded deciders,
  `verex_solve` the automatic leaf. Locals and quantified variables keep
  their source names; hypotheses are anonymous and selected by shape
  (`‹∀ i : Int, … › k h`).
- A function stating an in-body `assume` is verified of the reading in
  which its runs pass each assumption only where its condition holds
  (`Meanings.assumption`, `Term.assume`); its theorems take that its
  assumptions hold (`AssumptionsHold`). A caller using its theorem takes
  the same hypothesis; a caller inlining it assumes nothing of it.

## Rules of the implementation

- Freezing a mutable reference denotes its current value; the loan stays
  live and is resolved where its holder dies. A loop frame releases the
  current value of a reference the body writes through, and keeps the
  global memory unchanged unless the body reaches a global operation or a
  callee that may modify storage.
- A field selection whose operand is a mutable reference has no executable
  meaning, so the lowering never emits one: a field read through `&mut`
  selects from the dereferenced referent, a field borrow or assignment
  through `&mut` is a place rooted at the dereferenced local, and a write
  through a computed reference rests the reference in a hidden holder and
  writes the place through it. Variant payload fields are projection steps
  (`Proj.variant`), so `self.value := v` on an enum verifies like a struct
  field.
- The normalization names only simp sets. Ground evaluation (literal
  arithmetic, comparisons, casts, string equality, decided `dite`) is the
  set `vir_denote_eval`, so the guards of an operation at literal
  arguments are decided before its result is built. Array literals are
  updated at literal positions by simprocs.
- Everything `compileFunction` computes is evaluated by the kernel when a
  target's `compiled_eq` certificate is checked, so it reads only what the
  target's body reaches (roles are collected from the function root, never
  by a namespace-wide fold). The kernel reads an array through its list,
  so kernel-evaluated certificates iterate lists, read arenas through an
  `IndexedArena` built in one pass, and count sizes once
  ([`performance.md`](performance.md), "What was learned").
- Every computation of the runtime over the prepared unit that a proof
  cites is a kernel decision: the frame instantiation of a generic call at
  the runtime family is `Eq.refl` of the elaborator's value, stated through
  the callee namespace's view, never a Meta-level `rfl`.
- **Instantiation certificates by witness.** The runtime's
  `invocationTypeInstantiation` locates each instantiated compound type by
  a search over the namespace's type table. Instead of replaying the
  search, a certificate checks: once per namespace, a key map from the
  fingerprint of every erased type to the least index carrying it, whose
  correctness the kernel decides with one lookup and one comparison per
  entry (soundness uses only that `lookup` is a function of the key); per
  certificate, a witness per type index and a depth bounding the
  recursion, verified by a checker linear in the table. The soundness
  theorem relates `check … = true` to `instantiatePlaceFieldTypeFuel?`
  by induction on the fuel. Module
  `VerexIR/Validation/InstantiationCertificate.lean`. Every generated
  theorem is elaborated with `Elab.async` off, since an asynchronous proof
  that fails is committed as `sorry` and reported too late.
- The solver is a worklist: a `wp` over a marked loop takes the loop rule,
  over a callee the callee's theorem, over the recursive iteration the loop
  hypothesis; a binder, conjunction or conditional splits; only a goal
  whose head is `wp` is renormalized. Normalization is head-only: a bind's
  action is normalized and its continuation left untouched; when the
  action can branch the continuation is bound as a local definition, as a
  loop's is, so no branch renormalizes the rest of the program. Every goal
  the solver holds is normalized, so a step normalizes only the structure
  it creates.
- `Term.denote` and `Args.denote` rewrite by their equations, so a proof
  carries each step as an equation instance the kernel checks by
  instantiation; unfolding stays the fallback where an equation's type
  indices do not match at reducible transparency. The equation lemmas are
  realized under the package's default heartbeat budget, raised in
  `lakefile.toml`.
- A leaf clears every computation hypothesis first, adds the bounds of
  every range certificate in context as separate facts (never rewriting
  the certificate a term depends on), saturates with the context, splits
  the range-check conditionals, and decides by `omega`/`decide`. Signed
  quotients and remainders use width-specific range facts. A leaf whose
  deciders fail on a goal stating binders or premises is retried at an
  arbitrary instance within its own budget (`instanceAttemptHeartbeats`).
  A goal one position past the range of a quantified hypothesis splits on
  the one range premise omega does not prove (`verex_range_instance`).
- Vector membership and search results share one normal form, the
  position of the element; a leaf reading a search result splits it into
  `none` and `some found`; a lookup into an array with an element removed
  or inserted splits on which side of that position it reads, at integer
  positions. An in-bounds lookup the context does not state names its
  element, and position-quantified hypotheses are instantiated at every
  position the context reads.
- Saturation ends at a fixed point up to the order of the hypotheses, and
  a bound is asserted once per leaf, recorded by goal content.
- Inventories are simp sets, never explicit lists. `HList` and
  `variantCarrier` are not reducible, so every lemma whose right-hand side
  builds a row value is stated at the row type. One spelling per encoding:
  a lemma's right-hand side encodes through `τ.codec.encode`, never
  `NTy.encode τ`.
- `simp_all` of Lean 4.32 drops a hypothesis it modified in two rounds;
  the solver uses a vendored copy with the bookkeeping corrected
  (`verex_simp_all`, [`Denote/SimpAll.lean`](../../verex-ir/VerexIR/Proofs/Denote/SimpAll.lean)).
- A callee's post `encode x = literal` is read as `decode? literal = some
  x`, so the literal names the value. The leaf runs without error
  recovery, since `all_goals` under recovery aborts instead of letting
  `first` move on.
- The general leaf is one pipeline (`verex_pipeline`), each stage over the
  previous stage's goals; a round collects facts, splits and substitutes,
  then rewrites; state facts are retried after every round. A literal
  integer decoding is decided outright by a pre-simproc.
- A concrete state is computed rather than reasoned about. Substitution
  reaches what an equation fixes besides a local: an integer local the
  context pins to one literal, a vector local whose elements an equation
  states, and a caller's value an inlined generic callee's equation fixes
  (`NTy.eq_toSkolem_iff`). A contract call whose result the contract
  fixes to a few literal values is decided at the call, once for every
  leaf after it (`verex_call_result_cases`). A leaf whose positions range
  over a few literal values is decided value by value (`verex_split_range`,
  `verex_expand_ranges`). The discharger proves a side condition from the
  hypotheses connected to it through shared variables.
- Rows are a mutual family `NTy`/`NRow`/`NRows` (a nested inductive cannot
  derive decidable equality); an enum carries the distinctness of its
  variant names. Aggregate arguments are introduced destructured, one
  goal per variant. `do`-notation never appears in a denotation
  definition.

## References as prophecies

A `&mut` in the denotation is a prophecy pair, and the agreement is
restated up to prophecy resolution (the RustHorn model of
[`prophetic-references.md`](prophetic-references.md)).

### Model

- The carrier of `NTy.ref τ` is `τ.carrier × τ.carrier`: the current value
  and the prophecy, the value the reference will hold when it dies. There
  is no loan id.
- A borrow of a place `x.p` chooses a prophecy `π`; the reference is
  `(x.p, π)` and the lender holds `π` at `p` from then on. A mutable global
  borrow `&mut R[k]` is `(R[k], π)`, and the memory holds `π` at `k`.
- Reads and writes through a reference act on its current value.
- A loan's death (its `endLoan` marker) resolves the holder: execution
  continues only where `current = prophecy`. A loan consumed by a call
  argument resolves inside the callee.
- A `&mut` parameter arrives as `(c, φ)`. Every exit resolves each `&mut`
  parameter the function still holds. Moving a reference-typed local
  clears its slot, so a parameter moved into the result is resolved by
  the caller.
- A call passes values; a reborrowed argument is a borrow as above, so
  after the call the lender holds the argument's prophecy, which the callee
  has resolved. There is no write-back and no export step.
- A returned reference is an ordinary value: `reborrow(slot)` returns
  `(c, π)` with `slot = (π, φ)`; the exit resolves `π = φ`, so the caller
  receives `(c, φ)` while its lender already holds `φ`. The returned
  mutation carries the lender's future.

Choice and resolution are `Spec.choose` and `Spec.assume`; `wp` reads them
as a universally quantified value and a hypothesis. A local's borrow chooses
through `prophecyOf x p`, which is `Spec.choose` marked with the borrowed
local and path: its rule (`wp_prophecyOf`) gives the value with an inert
premise `Prophecy x p π`, which the walk consumes where it introduces `π`:
a step proof binds a whole local's prophecy as the value written back,
`x'` (`doc/plan/proof-language.md`, "Semantics and lowering"), any other
proof drops it. Both are generic in the value's type and carry no family,
so the rule needs no instance. A loan's death
resolves each of its holders that still holds a reference; a holder it
moved from resolves nothing. A loop that only writes through a reference
local keeps its prophecy. Callees are collected through every inlined
body, since a callee returning a reference is inlined. The rule-1 audit
forbids the loan predicates (`FreshStorageLoanIds`, `LoanDiscipline`,
`storageLoanTargetIn?`, `removeStorageLoan`) in verified artifacts.

### Agreement

`propheticMeaning unit handle f : HList f.params → Comp f.result.carrier`
is defined from the big-step meaning by a loan assignment and prophecy
resolution: `ok initial r final` holds when there are distinct loan ids for
the argument references and a start state agreeing with `initial` on
`globals` such that BigStep runs the arguments' current values with those
loans to results `rs` and state `s₁`, `r` carries `rs` with each returned
reference's prophecy, the export of every argument loan in `s₁.pending`
(returned loans' holes replaced by their prophecies) encodes the
argument's prophecy, and `final` is `s₁` with the loan fields of
`initial`; `aborts` and `undefined` likewise. `compileFunction_agrees` is
`Spec.Equiv (f.denote unit args) (propheticMeaning unit handle f args)`.

Intrinsic maps ([`intrinsic-maps.md`](intrinsic-maps.md)): the runtime's
values are read up to `normalize`, which replaces every well-formed value
of an intrinsic map owner by the canonical value of its abstract layout;
the runtime meaning is the quotient of big-step evaluation by `normalize`.

`satisfies_prophetic` transports a contract over the denotation to
`SatisfiesFunction` of its runtime form, which reads a mutable parameter's
exit as its export in the value view. The transport is proved, not
assumed.

### Contracts

- The contract of `verify f` is stated over `HList f.params` and
  `f.result.carrier` directly. In `ensures`, `old(p)` of a `&mut`
  parameter is its current value and `p` its prophecy.
- A returned reference is read in the value view: `ensures` holds under
  the premise that every returned reference's prophecy equals its current
  value.
- A contract may read a returned reference's final value,
  `final(result)`; such a contract drops the value-view premise. A callee
  returning `&mut` whose contract does not read `final` keeps the value
  view, so a caller reasons over its denotation through the agreement.
- References at the top level of a parameter or result, and components of
  a result tuple, are carried; a reference inside an aggregate is rejected
  by name.

## Recursion and cycles

A self-call denotes the callee's closed prophetic meaning, and a contract
proved by assuming it at the self-calls is sound because that meaning is
the least fixed point of the body: big-step derivations are finite.

- The denotation takes the meaning of calls as a parameter
  (`CalleeMeaning`). The closed denotation passes `closedMeaning unit`; the
  body of a member of a cycle at a slot passes `cycleMeanings unit members
  typeInstantiation self slot`. A slot (`CycleSlot`) is a member at the
  runtime family, or at a skolem family and type instantiation. A call to
  a member without type arguments means `self` at the caller's slot, one
  with type arguments `self` at the family and frame instantiation the
  call induces; every other call is closed. A function calling itself is a
  cycle of one.
- Assumed, beside the closed agreement: `compileFunction_least_cycle`, the
  closed prophetic meanings of a cycle's compiled members at every slot
  refine the least fixed point (`Spec.fixFamily`) of their denotations
  with the calls to members routed to the argument. Refinement, not
  equivalence: partial correctness needs only that every outcome of the
  closed meaning is a finite unfolding.
- Proved: `satisfies_cycle`, contracts per slot that hold of the members'
  bodies under the hypothesis that they hold of the calls to members hold
  of the closed meanings, by induction on the unfolding depth. A slot a
  proof does not cover carries `Contract.vacuous`.
- The cycle of a function is every function it reaches by calls that
  reaches it again. `verify` of any member verifies all of them together,
  each member's theorem over an arbitrary meaning per member satisfying
  its contract; they succeed or fail together. Every member is used
  through its contract, so each must be specified, not `pragma verify =
  false`, and all in one module. With a generic member, every member is
  proved at every family and instantiation and the runtime slots are the
  vacuous ones; a generic member's hypothesis is quantified over families
  and instantiations. A cycle through an unspecified function is rejected.
- The semantics and public theorems are elaborated synchronously: an error
  in an asynchronous proof leaves a `sorry` that only the audit reports.

## Generics

Generics follow the Move Prover's monomorphization (TACAS'22,
`move-prover/doc/paper21/design.tex`), with the fresh type carried as a
universally quantified carrier. A generic function is proved once at a
skolem instance plus the finitely many storage-aliasing instances; callers
use its contract and never prove an instance of their own.

- **Skolem family.** `class Skolems` gives, per type parameter index, a
  carrier, a tight codec to `RuntimeValue` whose encodings are `Plain`, and
  decidable equality. `NTy.param i` denotes `Skolems.carrier i`. Carriers,
  codecs, lending, the meanings, `Term.denote` and contracts are stated
  under `[Skolems]`; `Term`, `compileFunction` and the published artifacts
  do not mention it. The ground family (`Skolems.ground`) types literals;
  the runtime family (`Skolems.runtime`) is the public theorem's, and a
  function without type parameters is proved at it.
- **Type arguments.** The compiler takes a substitution for the function's
  type parameters: the skolem instance maps parameter `i` to `.param i`,
  an aliasing instance maps it to a concrete type.
- **Runtime instantiation.** `functionSpec` and `propheticMeaning` take
  the frame's type instantiation; `Term.denote` takes the caller's and a
  call passes `callTypeInstantiation`. Typed theorems quantify over it;
  `SatisfiesFunction` stays at `#[]`. A frame's generic families key
  through its instantiation, which the empty instantiation reduces away. A
  skolem carrier is inhabited, so a clause reading an absent resource
  reads a default; a call's type arguments are a row certified inhabited
  by the compiler.
- **Calls.** Terms carry calls with type arguments as `Term.callGeneric`.
  A generic call denotes the callee's meaning at the callee's own signature
  under the family its type arguments induce, with arguments and result
  transported between the caller's and the callee's carriers; on concrete
  rows the transports are identities the normalizer removes. The callee's
  theorem, over every family, applies by unification; a generic call at
  the empty instantiation instantiates its callee's frame by a
  kernel-checked literal. A generic function calling itself at its own
  type parameters is a cycle.
- **Verify.** `verify f` of a generic `f` proves the skolem instance over
  every family and instantiation whose skolem type ids avoid the aliasing
  types, and each aliasing instance: a modified memory unifying with a
  modified or accessed one, closed under combining unifiers as in the
  paper. The theorem for every instantiation is a proved case split, not
  the paper's meta-argument. Where a contract distinguishes the
  instantiations of one resource, the proof fails rather than assuming.
- **Agreement.** `compileFunction_agrees` and `compileFunction_least_cycle`
  are stated over every family and instantiation: a skolem value is only
  moved, compared by its encoding, stored, and lent as a `Plain` encoding.
- **Not carried by design.** A type argument that is a reference (Rust) is
  not `Plain`: such an instance is specialized. Type-dependent natives
  (`type_info`) are not carried.

## Discipline

- No new per-target agreement module, native generator, or shape-matching
  certifier. A construct `denote` does not carry is a negative check
  naming it, and a case of the induction that cannot be closed is such a
  construct.
- No `sorry` in `denote_agrees`; the named axioms stating it are removed
  by proving it, never extended.
- Cost is gated per target by `DenotePerformance.exp`; caps are not raised
  to count a port as done.
- Every `denote` case ships with its induction case. A false `ensures` on
  every new construct must fail. For every construct `denote` does not
  carry, the expected diagnostic names it.
