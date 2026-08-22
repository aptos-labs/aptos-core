# Poison-aware borrow checking and prophecy certificates

Status: design proposal

## Verdict

This is feasible without recreating compiler-v2's borrow checker.  It is not a
small addition, however, because the proposed rule is deliberately more
permissive than the ownership discipline currently assumed by Leaner's
prophecy translation.

An executable, intraprocedural checker is a medium-sized project.  A checker
with recursive call summaries, a small independently checked certificate, and
a theorem connecting accepted programs to prophecy semantics is a
medium-to-large verification project.

For one engineer already familiar with this repository, the expected effort is:

| Deliverable | Estimated effort |
| --- | ---: |
| Typed source-borrow view and intraprocedural prototype, including loops | 3--4 weeks |
| Calls, returned references, recursive SCC summaries, diagnostics | +3--5 weeks |
| Stable certificate format and small certificate checker | +2--3 weeks |
| Delayed-prophecy semantics and soundness proof | +4--8 weeks |
| Integration, negative tests, fuzz/differential tests, documentation | +2 weeks |
| **Complete first version** | **14--22 engineer-weeks** |

The lower end assumes the initial language remains the current Leaner subset:
no stored references, no nested reference types, no function values, vectors
abstract all indices, and returned references derive from parameters.  Precise
vector-index reasoning, imported/native function inference, or matching all of
compiler-v2 would extend the schedule.

This estimate is substantially lower than a greenfield implementation because
`MoveModel.IR.RefElim` already has most of the algorithms which can be factored
into source-independent libraries:

- an access-path borrow graph (`BNode`, `BStep`, `BEdge`);
- backwards liveness and per-instruction death points;
- forward CFG fixpoints with union joins and post-fixpoint validation;
- parameter-relative return derivations;
- call summaries and Kleene iteration for recursive functions;
- a large correctness development around reference elimination.

Those algorithms should be factored and reused, but the authoritative
certificate must be about retained source, not about `MoveModel.IR`.  The
current rejection policy should not be reused unchanged: it eliminates
immutable references first and rejects overlapping mutable children eagerly,
whereas this design keeps both kinds visible and delays failure using static
poisoning.

## Intended policy

The semantic model is inspired by the access-path trees and lazy poisoning in
[Defense-in-Depth Runtime Safety in Move](https://arxiv.org/html/2606.18064v2),
especially section 3.3.  The paper describes a dynamic checker for one executed
path.  Leaner needs a finite static abstraction covering every CFG path and a
certificate which the Lean kernel can check.

This design intentionally does **not** claim equivalence with compiler-v2 or
the Move bytecode verifier.  It defines a source-verification acceptance rule.
Compiler-v2 and the bytecode verifier remain independent downstream checks.

### Places and overlap

A reference denotes a place in an access-path tree:

```text
Place = Root / Step*
Root  = local | global-resource-family | reference-parameter
Step  = field-offset | any-vector-index
```

Two places overlap when their roots may be equal and their paths are equal or
one is a prefix of the other.  Distinct field offsets are disjoint.  Initially,
all elements of a vector share `any-vector-index`, as both the current
reference-elimination pass and the runtime checker already use a conservative
index abstraction.

Global roots are initially keyed by resource family, not by an address value.
Two borrows of the same resource family may therefore alias even when their
address locals differ.  This is conservative and avoids putting an arithmetic
alias prover in the certificate checker.

### Mutable handles and delayed activation

Creating `&mut p` produces an *unactivated mutable handle*.  It records a
place, but does not yet create an independent prophecy loan.  Overlapping
unactivated mutable handles may coexist.

A mutable handle is activated by a destructive use:

- writing through it;
- passing it to a call whose summary may write through that parameter;
- invoking a vector/native operation summarized as destructive.

Activation creates the prophecy-backed loan.  An already activated handle can
be written repeatedly.  At activation or a subsequent destructive use, every
other live mutable handle at an overlapping place is statically poisoned.  A
poisoned handle may be moved or dropped, but may not be read, written,
reborrowed, returned, or passed to a callee.

Equivalently, at every use of a mutable handle, no *other* handle which could
have performed an overlapping write may have poisoned it.  This is the static
counterpart of the paper's lazy failure rule.

The distinction between handle creation and activation is necessary.  The
current `withMutation` semantics opens a separate prophecy at each mutable
borrow.  Two overlapping independently reconciled prophecies can demand
different final values even if only one handle is used.  Delaying the prophecy
until the unique destructive lineage is selected avoids that contradiction and
admits the temporary aliasing which poisoning is intended to allow.

### Immutable references

The initial rule is intentionally stronger than overlap-only exclusion:

> A prophecy may be activated only when no live immutable reference exists
> anywhere in the same access-path tree.

Conversely, creating an immutable reference in a tree with an activated
prophecy is rejected.  Immutable-to-immutable reborrows remain valid.  Freezing
a mutable reference consumes that mutable handle and creates an immutable
reference at the same place.

Using tree-wide rather than path-overlap exclusion makes the certificate
useful to the proof layer: an activated tree has one mutable prophecy regime
and no observations which can become stale.  The access-path domain permits
relaxing this to overlap-only later if examples justify the added proof burden.

### Reborrows

Ancestor and child mutations are one ownership lineage, not competing aliases.
A child prophecy suspends the overlapping part of its parent and reconciles
back into it at child death.  A parent may be used to create another child, but
ordinary reads and writes through the parent are unavailable while an
overlapping child is active.

Disjoint sibling children may be activated independently.  Overlapping sibling
handles may be created, but once one is activated the others become poisoned;
only the activated child's reconciliation contributes to the parent.

This retains the current nested-prophecy account of field and vector reborrows
while replacing the current eager rejection of overlapping siblings with the
poisoning rule.

### Lifetime and escape

Liveness determines when a handle dies and when an activated prophecy is
reconciled.  A returned reference must derive from a reference parameter along
a summarized path.  References rooted in locals or globals cannot escape a
function in the first version.  References still cannot be stored in structs,
enums, vectors, or global memory.

## Analysis boundary

The authoritative checker must run before compiler IR.  `verify` proves the
retained source body by translating it to `Move.Semantics.Spec`; an IR-only
certificate would establish a property of the compiled artifact, not of that
source specification.  Using it in a source proof would require the currently
missing source-to-IR compiler-correctness theorem and would make the
certificate circular as a near-term proof premise.

Introduce a small typed **source borrow program** derived from `move_source`.
It is a checked view of the retained declaration, not the executable compiler
IR.  It preserves source names and ranges and contains only the information
needed by borrow analysis:

```text
SourceBorrowProgram:
  typed parameters, locals, and returns
  source-level borrow/read/write/freeze/owner events
  structured branches and loops (or a source-point control graph)
  direct call identities and source call arguments
  source ranges for every event and control edge
```

Both the borrow checker and prophecy-spec generator consume this same view:

```text
retained Leaner source
  -> typed SourceBorrowProgram
       -> borrow analysis + checked SourceBorrowCertificate
       -> prophecy-scope plan -> generated sourceSpec -> verify
  -> ordinary Lean elaboration / LCNF -> LIR -> semantic IR -> XIR
       -> optional transported/rechecked IR certificate
```

The source borrow program should be produced once by a syntax-directed,
type-aware elaborator and cached with the retained declaration.  The prophecy
translator must not independently rediscover loan scopes from raw syntax.  It
uses the certificate's activation, lineage, call-summary, and death facts.
This makes `WellBorrowed` directly available when proving `f.sourceSpec`.

This source view still needs normalized control points so loops and joins have
a finite fixpoint domain, but it must remain traceable one-to-one to retained
source constructs.  It should not depend on LCNF, temporary locals, block
splitting, reference elimination, or positional resource/function IDs.

The compiler path remains independent initially.  A later compiler-correctness
stage transports source places and loan sites to LIR/IR and either proves the
transport or rechecks a corresponding IR certificate.  That downstream check
is valuable for proving emitted code, but it is not the evidence used to admit
source prophecy proofs.

## Abstract domain

The analysis state at a program point is finite:

```text
RefOrigin:
  root          : RootPattern
  path          : PathPattern
  mutability    : immutable | mutable
  lineage       : optional activation/reborrow site

RefSlot:
  origins       : finite set RefOrigin
  phase         : unactivated | activated | suspended | dead

PoisonEdge:
  writer        : RefOrigin
  victim        : reference local / origin

BorrowState:
  reference slots
  active prophecy sites
  parent/child lineage edges
  conditional PoisonEdges
```

A `PoisonEdge writer victim` means that the victim is invalid if its possible
place overlaps the writer's place.  Keeping the writer rather than collapsing
immediately to `poisoned : Bool` is important for calls: a function may be safe
provided two parameter-relative places are disjoint, and the caller may be able
to prove that from concrete field paths.

At a use of the victim, each possible poison edge is discharged in one of two
ways:

1. the paths are statically disjoint; or
2. the checker emits a parameter-relative separation requirement into the
   current function summary.

If neither is possible, the use is rejected.  For a non-parameter local root,
there is no caller which can satisfy a requirement, so uncertain overlap is an
error.

Joins union possible origins, poison edges, and active sites.  A strong update
to a reference local removes facts about the old value.  The ordering is set
inclusion, giving a finite-height may-analysis.  Paths are bounded by the
finite, non-recursive Move type shape; an `unknown-suffix` widening should be
included defensively and overlaps every compatible descendant.

## Transfer rules

The following rules define the first implementation.  Every rule operates only
on source-live references; dead references are removed by liveness over the
source borrow program.  The implementation may reuse the generic liveness
algorithm, but not liveness results computed after compiler lowering.

### Borrow

- `borrowLoc`, `borrowGlobal`, `borrowField`, and `borrowVecElem` create an
  origin for the destination.
- An immutable creation requires that the tree has no activated prophecy.
- A mutable creation is unactivated and may overlap other mutable handles.
- A field or element reborrow records its parent lineage.
- Malformed reference/value type combinations are rejected independently of
  poisoning.

### Read and reborrow

- Reading, freezing, reborrowing, returning, or passing a reference first
  discharges every poison edge targeting it.
- Reading/writing a parent through a path covered by an active child is
  rejected; creating a sibling child is handled by the path rules instead.
- Immutable reads do not change the state.

### Write

- A write first proves that the writer is not poisoned.
- If this is its first destructive use, activate its prophecy and require that
  no live immutable origin may share its tree.
- Add poison edges from the writer to every other live mutable origin whose
  place may overlap, excluding its suspended ownership ancestors.
- Mark overlapping sibling/independent handles poisoned conditionally.
- Preserve the writer's activation so repeated writes through the same handle
  are valid.

### Move or overwrite an owner

Moving or overwriting a local/global root is a destructive operation on the
root place.  It requires no active prophecy and no usable reference in the
tree.  The first version should reject rather than try to poison owner-rooted
references, because prophecy write-back would otherwise target an invalid
owner.

### Death

- Dropping an unactivated or poisoned handle has no semantic effect.
- Dropping an activated child reconciles its prophecy into its parent.
- Dropping the outer activated handle reconciles into the local/global owner.
- An activated loan closes only after all live returned/derived descendants
  have closed or transferred to the caller.

### Abort

Abort needs no reconciliation into observable transaction state.  Certificate
soundness must nevertheless show that no poisoned reference was used on the
executed prefix.  This matches the current prophecy semantics, where pending
global write-back is unobservable after transaction rollback.

## Function summaries

Calls must be checked without inlining.  A finite `BorrowSummary` should
contain:

```text
BorrowSummary:
  reference parameter kinds
  required separation constraints between parameter-relative places
  parameters/paths which may be read
  parameters/paths which may activate or write
  returned-reference derivations (parameter, path, mutability)
  returned activation/poison status
  global trees read, activated, or invalidated
```

At a call site the checker:

1. maps parameter-relative summary paths to caller origins;
2. checks the callee's separation and immutable-tree requirements;
3. treats every summarized mutable write as a destructive use in the caller,
   adding poison edges to other overlapping caller handles;
4. creates result origins from the return derivations;
5. transfers any activated returned lineage back to the caller.

Passing a mutable reference does not automatically require compiler-v2's
exclusive-call rule.  A read-only callee can receive overlapping mutable
handles.  A write-capable callee activates the relevant argument; conflicts are
then governed by its summary.  This is one of the intended differences from
compiler-v2.

Unknown or native callees need an explicit trusted summary.  The safe fallback
is: every mutable parameter may write, every immutable parameter may read, no
reference is returned without a declared derivation, and all declared global
effects may occur.

## Loops and recursion

### CFG loops

Use the existing worklist/fixpoint algorithm over source control points.  Each
loop header/join certificate supplies an entry `BorrowState`.  For every source
control edge, the checker verifies:

```text
transfer(block, entry[block]) <= entry[successor]
```

The analysis may compute a least fixpoint, but soundness depends only on this
post-fixpoint check.  This follows the existing `liveStable` and `graphStable`
pattern and prevents a fuel-exhausted under-approximation from being trusted.

A poison edge created in one loop iteration therefore reaches later
iterations.  A loop which writes through one overlapping handle and later uses
another is rejected even when the two operations occur in different
iterations.

### Recursive functions

Compute summaries per strongly connected component by monotone Kleene
iteration.  Summary effects and requirements grow by union; return-derivation
paths use the finite path domain and widening.  The certificate checker then
validates every summary as a simultaneous post-fixpoint of the SCC bodies.

This is stronger than merely checking that the summary generator reached
equality: an externally generated or cached summary can be accepted safely if
the small checker validates the post-fixpoint equations.

## Certificate design

The certificate should contain facts, not proofs with large terms:

```text
SourceBorrowCertificate:
  version
  retained-source declaration digest/identity
  function summaries
  per-function source-point entry states
  activation sites and loan lineage
  call-site summary instantiations
  return derivations
```

The checker recomputes source-event transfers, liveness use/death boundaries,
path overlap, and all control-flow/SCC inequalities.  It must not trust cached
booleans such as "these paths are disjoint".  Certificate generation can be
optimized or moved outside Lean later without increasing the trusted base.

The API should have this shape:

```lean
def analyzeBorrows (p : SourceBorrowProgram) :
    Except BorrowError SourceBorrowCertificate

def SourceBorrowCertificate.check
    (p : SourceBorrowProgram) (certificate : SourceBorrowCertificate) :
    Except BorrowError Unit

def WellBorrowed (p : SourceBorrowProgram) : Prop :=
  exists certificate, certificate.Checks p

theorem SourceBorrowCertificate.sound
    (h : certificate.Checks p) : PoisonSafe p
```

Generated modules can expose compact declarations such as:

```lean
def MyModule.f.borrowProgram : SourceBorrowProgram := ...

def MyModule.f.borrowCertificate : SourceBorrowCertificate := ...

theorem MyModule.f.wellBorrowed : WellBorrowed MyModule.f.borrowProgram :=
  SourceBorrowCertificate.soundChecked (by decide)
```

The exact theorem should avoid making users unfold the certificate.  Proof
automation should consume `wellBorrowed` as evidence that generated prophecy
scopes are legal.

## Soundness argument

The proof should be staged rather than coupled immediately to all of source
verification.

### 1. Concrete poison monitor

Define a small operational monitor over `SourceBorrowProgram` which carries
concrete access paths, handle identities, poison status, and active prophecy
lineage.  It follows the policy above and has a distinguished reference-safety
failure.

This monitor is a specification, not production runtime code.  It is close to
the paper's shadow semantics but includes delayed prophecy activation and the
tree-wide immutable condition.

### 2. Abstract interpretation soundness

Define a relation from concrete monitor states to `BorrowState`.  Prove each
source-event transfer preserves it and that an accepted transfer cannot take a
concrete reference-safety failure step.  Lift this through source control
edges, structured loops, calls, and recursive summary post-fixpoints.

The main result is:

```text
certificate checks
  => every monitored execution is poison-safe
```

### 3. Prophecy compatibility

Prove that every activation in a poison-safe execution can be represented by
one `withMutation` scope:

- the owner exists and is not invalidated;
- no immutable observation exists in its tree;
- overlapping competing mutable handles cannot subsequently activate;
- child activation/reconciliation composes through the existing lenses;
- the liveness frontier closes every normal loan exactly once.

This is where delayed activation is connected to the current `Mutation` and
`ProphecyLoan` definitions.  It may be useful to introduce an explicit
`PotentialMutation` semantic object for an unactivated handle, with no
prophecy field, and a one-way `activate` operation producing `Mutation`.

### 4. Source-proof integration and downstream transport

Make the source-spec translator consume the certificate's activation and death
plan instead of independently guessing scopes from syntax.  Prove directly
that the planned `withMutation` scopes refine the monitored source borrow
program.  This makes the certificate useful in Move program proofs immediately
and does not depend on compiler correctness.

Separately, define a mapping from source roots, paths, and sites to LIR/IR
operations.  The future source-to-IR correctness theorem must show that this
mapping preserves borrow events, or validate a transported IR certificate.
Only this second theorem transports the already borrow-checked source contract
to emitted XIR/bytecode.

## Implementation plan

### Phase 0 — executable policy examples

- Write accepted and rejected examples before defining the domain.
- Settle delayed activation, tree-wide immutable exclusion, same-place mutable
  aliases, freeze, owner invalidation, and returned activated references.
- Encode the concrete poison monitor for a straight-line, single-function
  source borrow program.
- Compare examples with the Aptos runtime reference checker, documenting the
  deliberate differences.

Exit criterion: each informal rule has an executable positive and negative
example, including the overlapping-handle case which current prophecies cannot
represent eagerly.

### Phase 1 — factor shared infrastructure

- Move reusable `BNode`, `BStep`, access-path overlap, graph operations, and
  fixpoint/liveness algorithms out of `RefElim.Transform` into small generic
  modules.
- Define the typed `SourceBorrowProgram` elaborator over retained `move_source`.
- Preserve immutable-reference identities, source names, and source ranges.
- Make the source-spec translator and checker share source-point identifiers.
- Keep `RefElim` behavior unchanged under the factored APIs.

Exit criterion: a retained function produces a stable source borrow program;
the existing reference-elimination and transactional suites pass unchanged.

### Phase 2 — intraprocedural certificate checker

- Implement `BorrowState`, poison edges, joins, and transfer rules.
- Compute source liveness for deaths and reconciliation frontiers.
- Check straight-line functions, branches, and structured loops.
- Validate source-point entry states as a post-fixpoint.
- Emit exact, source-positioned diagnostics.

Exit criterion: loops converge; mutating a computed block state causes the
certificate checker to reject it; no safety decision relies on generator fuel.

### Phase 3 — calls and summaries

- Define `BorrowSummary` and call instantiation.
- Infer read/write/activation effects and returned derivations.
- Add SCC discovery and simultaneous summary iteration.
- Validate recursive summaries as post-fixpoints.
- Add explicit summaries for supported vector/native operations.

Exit criterion: mutable references can be passed through ordinary and mutually
recursive calls; conflicts created in callees surface at caller source sites.

### Phase 4 — delayed prophecy semantics

- Add unactivated mutable handles to the logical semantics.
- Activate `Mutation`/`withMutation` only at destructive operations.
- Use the certificate's lineage and death points for nested field/vector/global
  scopes.
- Extend mutable-call semantics to use the callee summary rather than assuming
  every `&mut` parameter is independently exclusive from call entry.

Exit criterion: accepted poisoning examples have generated `sourceSpec`s and
ordinary contract proofs; the former eager-prophecy contradiction is absent.

### Phase 5 — soundness and proof API

- Prove monitor safety from checked intraprocedural certificates.
- Extend the proof across call summaries and recursive SCCs.
- Prove prophecy compatibility and expose `WellBorrowed` to `verify`.
- Ensure certificate checking remains small and independent of the analyzer.

Exit criterion: source verification requires checked borrow evidence, and a
deliberately corrupted certificate fails before proof generation.

### Phase 6 — integration quality

- Add negative language tests under `Move/Tests/Negative/Borrows.lean` with
  exact messages.
- Add positive verification tests for certificates and prophecy proofs.
- Add source/IR event-correspondence tests without treating them as the proof
  of source well-borrowedness.
- Add transactional cases ensuring accepted code still passes compiler-v2 and
  the bytecode verifier.
- Differentially test the static checker against the concrete monitor on small
  generated CFGs.  Runtime-checker differences are classified, not blindly
  treated as bugs.
- Benchmark analysis and certificate-check time with `scripts/bench-proofs.sh`.

## Required tests

### Accepted

- overlapping mutable handles are created; only one is activated and the other
  is dropped unused;
- repeated writes through the same activated handle;
- disjoint sibling field writes;
- a child write followed by parent use after reconciliation;
- a read-only callee receives mutable handles which may alias;
- a write-capable callee receives one handle and returns its final value;
- a loop carries one activated handle without introducing an alias conflict;
- a recursive summary propagates a parameter-relative write;
- abort with an active prophecy rolls back observable global write-back.

### Rejected

- write through one overlapping handle, then read/write/reborrow/pass the
  poisoned handle;
- an immutable reference exists elsewhere in the tree at activation;
- an immutable reference is created while a tree prophecy is active;
- parent use while an overlapping child is active;
- owner move/overwrite with live handles;
- poison created on one branch and victim used after the join;
- poison created in one iteration and victim used in a later iteration;
- two call arguments violate an inferred separation requirement;
- recursive summaries fail to reach a validated post-fixpoint;
- returned references escape a local/global root or lack a derivation.

### Certificate and proof failures

- missing block state;
- under-approximated join;
- omitted call effect;
- false disjointness claim;
- invalid activation/death ordering;
- under-approximated recursive summary;
- certificate generated for a different module/version.

## Risks and containment

### Prophecy activation timing

This is the largest semantic change.  Keeping eager prophecy creation would
make the relaxed overlapping-handle policy either unsound or vacuous.  The
prototype must validate delayed activation before substantial proof work.

### Conditional alias requirements

Tracking parameter-relative poisoners is more complex than a Boolean poison
flag, but avoids rejecting useful call patterns.  If it proves too costly, the
first milestone can conservatively require distinct trees for all mutable
parameters of write-capable functions while retaining the same certificate
format.

### Join precision

Union joins can report false conflicts after mutually exclusive branches.  Do
not add path conditions to the trusted checker initially.  If examples require
more precision, add finite disjunctions of states with a strict cap and widen
to their union.

### Recursive path growth

Type-bounded paths should make the lattice finite for the current language.
The checker must still implement `unknown-suffix` widening and verify the final
post-fixpoint rather than trust an iteration bound.

### Proof size and build performance

Keep the checker data-oriented and prove generic transfer lemmas once.  Do not
emit one large theorem term per instruction.  Benchmark certificate reduction
early; if kernel reduction is expensive, use a reflected checker theorem with
compact arrays and finite sets.

### Duplicate analyses

Borrow checking and reference elimination must not evolve separate notions of
paths, liveness, summaries, or death.  Factor these first and make reference
elimination consume the checked certificate where practical.

## Initial decisions

- The checker defines Leaner source-verification safety; it does not duplicate
  compiler-v2 acceptance.
- Mutable borrows create handles; destructive use activates prophecies.
- Poisoning is lazy: poisoned handles may move or die, but cannot be used.
- Immutable exclusion is tree-wide for the first implementation.
- Dynamic vector indices alias conservatively.
- Global locations alias per resource family, independent of address values.
- Calls use inferred, checkable summaries; recursion uses SCC post-fixpoints.
- Returned references must derive from reference parameters.
- The certificate is checked independently and is available as proof evidence.
- The authoritative analysis runs on a typed view of retained source, before
  compiler IR, immutable-reference erasure, and reference elimination.
- An IR certificate is a downstream transport/recheck, never the premise which
  admits a source prophecy proof.

## Open questions before implementation

1. Does "no immutable references anywhere in the tree" intentionally include
   immutable references to statically disjoint sibling fields?  This plan says
   yes.
2. Should a call activate every mutable parameter conservatively, or may a
   summary distinguish read-only mutable parameters?  This plan recommends the
   latter.
3. May an activated mutable reference itself be returned, transferring its
   prophecy to the caller, or should the first version require activation to
   close before return?
4. Should public/imported functions ship checked certificates, trusted summary
   declarations, or both?
5. Is resource-family global aliasing sufficiently precise, or is symbolic
   address equality needed for common programs?

Questions 1 and 3 affect the semantic theorem and should be settled during
Phase 0.  The remaining choices can be refined without changing the core
poisoning model.
