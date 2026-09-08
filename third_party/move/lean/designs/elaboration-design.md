# Leaner: Elaboration, Verification, and Execution from LIR

## Status and scope

This document defines the migration from the current Leaner source-specific
elaboration and verification path to a single path driven by validated
Leaner IR. It builds on the shared representation in
[`lir-design.md`](lir-design.md); it does not redefine the LIR
syntax or the frontend interchange format.

**Reframing (2026-08-27):** the `move`, `move-model`, and `transpiler`
packages were deprecated wholesale as reference-only; nothing current links
them, and the Move exchange frontend now lives in `LeanerMove.Frontend`.
This changes the migration story this document was written around: there is
no longer a live direct elaboration path to shadow (M3), migrate away from
(M6), or retire incrementally (M7) — the old path is frozen as an oracle and
its corpus is mined by copying tests, not by switching defaults. The spine's
remaining deliverables are the M2 tail (executable coverage of the corpus
plus the owed metatheory: determinism, completeness up to fuel,
preservation/no-stuck) and M4–M5 (contracts, calculated WP, and
borrow/state/loop/modular verification), targeting the LeanerLang surface and
the e2e corpus. Separately, the validation pipeline work (single-run
validate, authoritative resolution and unification typing, analyses run once
at validate with certificates on `ValidatedUnit`) landed 2026-08-27;
statements below that place authoritative typing in semantic preparation
describe the superseded arrangement — `validate` now owns typing and the
initialization/borrow analyses, and preparation filters capability.
The verification half of M4–M5 is carried since 2026-09-01 by
[`certifying-execution.md`](certifying-execution.md) (the frame-free row
route); this document stays authoritative for the runtime model, the
big-step semantics, the interpreter, and the correctness obligations.

**Implementation status:** M0 and M1 are implemented. The neutral package has an
exhaustive core semantic inventory, versioned profile-semantic registries,
private-constructor `ExecutableUnit` and `VerifiableUnit` preparation gates,
reachable-node capability checks, a first executable-literal typing check,
and stable location-bearing diagnostics. The Move profile classifies every
tag admitted by its structural schema and tests that the two inventories stay
in sync. M1 adds first-order runtime values, frames and reserved stores, pure
leaf operations, a fuel-free structured big-step relation, and a source-aware
fuelled interpreter for the complete M1 control subset. `run_sound` proves
that every successful interpreter invocation has a big-step derivation;
fixtures cover values, tuple and range patterns, assignment, branches,
structured loops, constants, direct calls, normal returns, multi-argument
throws, stuck diagnostics, call stacks, and authored byte ranges. M2 is in
progress: runtime values now include vectors, nominal values, closures, and
references; concrete heap/global references share nested-place machinery;
typed core pure operations, closure invocation, returned references, keyed global
storage, and profile-selected throw finalization execute in both the
interpreter and big-step semantics. The Move frontends now emit core global
and primitive operations rather than profile tags. Fixed-width arithmetic
distinguishes modular operations from explicit `checked*` operations carrying
their overflow `ThrowKind` and overflowing operations returning the modular
value plus a Boolean flag; Move emits checked arithmetic with `abort`.
Semantic preparation checks the closed primitive vocabulary's operand and
result types, including tuple shape, vector element/fixed-length obligations,
and source-located integer, Boolean, integer/Boolean comparison, indexing, and
slicing errors.
Raw validation rejects zero-width integers and negative or non-integer fixed
vector lengths before any semantic consumer can observe the type table.
Structural validation also requires canonical namespace paths to be unique,
requires each top-level declaration's interned qualified name to belong to its
containing namespace, requires nominal declarations to be exclusively struct-
or enum-shaped, and rejects duplicate fields and variants along with duplicates
within every resolver lookup family rather than allowing resolution to pick the
first ambiguous target. Associated-item ownership is bidirectional: every item
must occur exactly once in its owner trait, whose item names are unique within
the separate type and value namespaces. Local trait implementations bind only
that trait's items and must provide every item without a default.
The complete structural type graph, including logical type/resource-domain
arguments, is bounds-checked and acyclic before semantic preparation.
It also resolves local, dereference, vector-index, constant tuple-index,
non-generic nominal-field, and enum-downcast places far enough to check
read/move/copy results, writes, mutable borrows, assignments, drops, and
assertions before execution. Nominal-place validation resolves declaration,
variant, and field identities through the shared compilation-unit tables;
invalid projections retain the location of the operation using the place.
Direct calls are resolved during preparation and checked against callee
parameter and packed-result types after structural type/lifetime substitution.
Closure construction applies the same substitution to captured parameter
prefixes and the residual function type; invocation checks the callable,
argument vector, and result type. Generic bodies remain single LIR bodies.
Direct calls and closure construction also require the caller and target to
share a profile until an explicit validated boundary-adapter representation
exists.
Constructor/destructor and data-operation preparation resolves nominal targets,
variants, field arity, instantiated field/result types, and nominal result
arguments. Generic
nominal fields are resolved recursively through already interned structural
and nominal instantiations, including reference-bearing and nested generic
fields. A frontend must therefore intern the concrete field types used by its
nominal instantiations. Field selection/update substitutes the operand's
concrete generic arguments before checking result and replacement types; enum
selection and tests validate target shapes and common variant-field types.
Recursive semantic type traversal also checks that body-only type parameters
and parameter lifetimes resolve to an in-scope binder of the expected kind;
the global type arena cannot make an unscoped body annotation valid.
Generic arguments and both sides of lifetime-outlives predicates participate
in the same semantic feature and lifetime-scope traversal.
Resource-domain types resolve their named nominal declaration through the
compilation-unit/import boundary and check the encoded type arguments against
that declaration's generic binders before the M4 logical boundary.
Keyed global operations check Boolean/Unit results, resource values, and
borrowed resource referents. The Move profile additionally requires an address
key and a resource type with `Key`; neutral/Rust keyed globals retain their
profile-owned key and storage rules.
Core `Copy`, `Drop`, `Store`, and `Key` constraints are checked on concrete
generic type arguments. Move abilities propagate structurally and nominal
types use their declarations; Rust has implicit `Drop`, scalar/shared-reference
`Copy`, and explicit nominal/function `Copy`. Place/value copy and explicit
drop are rejected when the required ability is absent. Trait predicates and
checked selection evidence remain separate from this first-order ability pass.
Every trait reference nevertheless resolves to an owned declaration during
preparation, and its generic argument arity, kinds, and first-order ability
constraints are checked without selecting an implementation or fabricating
evidence. Structural validation rejects cycles in the locally owned supertrait
graph; external supertraits remain leaves until dependency interfaces carry
authoritative declarations.
Associated type/constant equality predicates additionally require their item
to belong to that declaration and to occupy the matching associated namespace;
the item ID is bounds-checked in the referenced owned namespace rather than the
predicate's source namespace. First-slice associated constant literals are
checked against the declared type after trait-generic substitution.
Normalization still waits for checked implementation evidence.
Implementation binding completeness, ownership, kind, and associated-constant
typing resolve in the implemented trait's namespace, including declared
cross-namespace imports; only implementation selection and evidence remain
deferred.
Declared struct and enum abilities are also checked against every payload
field, recursively using the declaration's generic bounds. Move `Key` checks
field `Store`; generic-to-generic instantiation reuses the caller's bounds.
Constructor patterns resolve the same nominal declaration and variant as
constructor calls, check generic arity and abilities, and substitute declared
field types before comparing their child-pattern types.
Structured control preparation tracks loop result types: `break_` and
`continue_` must select an enclosing loop, break values must match that loop,
result-less blocks which can fall through must have Unit type, while `if`
expressions without `else` and pattern assignments must have Unit type;
loop bodies which can fall through must likewise have Unit type.
Non-fallthrough expressions remain polymorphic through abrupt control. A
function body that can fall through must have its declared packed
result type. These checks remove the corresponding
interpreter-stuck or preservation failures from prepared units.
Constant declarations are checked against their initializer expression type;
constant-use nodes resolve their qualified target and match its declared type.
Associated-constant defaults are checked against their declared associated
type at the item location. Local implementation bindings substitute the
implemented trait's arguments before checking associated-constant values.
Associated-method defaults and local implementation bindings must also resolve
to owned function declarations; their full substituted-signature comparison
remains part of the checked dictionary boundary.
Generic nominal arguments are compared structurally across owned namespaces,
so source locations on equivalent type uses do not affect type identity. Both
failures retain the authored declaration or use-site `LocId`.
Executable preparation also performs path-sensitive definite-initialization
analysis over structured bodies. Parameters initialize the leading locals;
pattern bindings and direct-local writes initialize, while direct-local moves
and drops consume. Branch joins retain only initialization common to every
fall-through path, and loop back-edges are iterated to a finite fixed point.
Uninitialized reads are therefore rejected at their authored `LocId` before
the interpreter receives an `ExecutableUnit`.
The declarative `ValueHasType` judgment uses the same representable-range
predicate for runtime integers, so an out-of-range fixed-width value cannot be
used as a preservation witness merely because its type-table entry is integer.

Intrinsic declarations pass a shared structural graph check before they
reach any semantic consumer: owners and binding targets resolve to the right
declaration families in the owner's namespace, while duplicate owners and
roles and shared targets are rejected with source and related locations.
The Move profile now closes the 62 transported map roles and checks model and
role vocabulary, target category, key/value owner shape, required roles, and
role dependencies. It also matches every target against the exact
owner/key/value, reference, tuple, vector, nominal-wrapper, integer, and
logical-number signature alternatives declared for that role.
Every reachable nominal type use now resolves its struct/enum declaration and
checks generic arity, binder sorts, and ability constraints. The scan carries
the enclosing function, struct, trait, or implementation generic scope through
nested types, so `Box<T>` is accepted exactly when the active bounds establish
the requirements of `Box`.
Expression-level type parameters and parameter lifetimes are also checked
against the enclosing binder sequence during semantic preparation. This is a
backstop beyond raw arena bounds checking, which cannot assign one global scope
to an interned type table.
Move addresses, signers, and address constants are likewise core nodes rather
than residual profile tags. Specification conditions, namespace invariants,
axioms, and all four quantifier forms are typed core nodes shared with the
planned Rust specification frontend. Verification preparation now traverses
specification-function signatures/bodies/contracts, specification-variable
types/initializers, namespace invariants, and struct contracts before emitting
their deliberate M4 unsupported boundary. Predicate-valued conditions require
Boolean expressions, while let, decreases, abort-code, emit, and update values
retain their data types. Quantifier filters and universal/existential bodies
and results are Boolean; choice result and domain typing remain at the M4
boundary. Named emit conditions are checked as Boolean auxiliaries.
Specification-function call operations resolve their declaration and check
profile, generic arguments, parameters, and packed results before that same M4
logical boundary.
The shared checker also enforces result-slot bounds/types, type-domain
instantiations/results, correlates resource-domain operations with their
nominal type arguments/results, and enforces the fixed nullary/unary result
shapes of `old`, `wellFormed`, and `abortFlag`. Trace, no-op, and source-level
bit-vector/integer conversion nodes are unary and type-preserving, matching the
existing source backend's representation-independent rendering.
Specification vector construction, update, concatenation, containment,
index-of, in-range, and range operations check their fixed arity and
vector/element/index/result relationships before M4 interpretation. Their
indices use the logical unbounded `num` type rather than executable vector
index types. Generic specification builtins also require their single type
instantiation to agree with the transported operand/result element type.
Range membership, fixed-width maximum values, and empty event stores likewise
check their closed core result shapes before logical interpretation.
State-domain construction and the Boolean event-store inclusion relations also
check their fixed domain/result shapes.
Logical resource publish, remove, and update operations check their single
resource type argument, address/value operands, Boolean result, and the Move
profile's `Key` constraint before M4 interpretation.
Logical global reads likewise check their resource type/result and the Move
profile's address-key and `Key` constraints.
Logical `canModify` checks its resource type argument, address key, Boolean
result, and Move `Key` constraint.
Internal state-anchor markers, anchored wrappers, and inline-call summaries
check their transported Boolean/identity shapes. Event-store inclusion is
unary, matching the Move model expressions it transports; event-store
extension checks its 3/4-operand store and optional-condition shape. The
nullary abort-code operation produces logical `num`.
Specification functions and variables must belong to their namespace's
semantic profile, matching the existing executable-function and intrinsic
ownership invariant.
Bitwise OR, AND, and XOR are typed executable core primitives for booleans and
fixed-width integers; fixed-width integers additionally support complement.
Integer forms normalize operands to the result width and reconstruct signed
results from the same two's-complement bit pattern. Checked left/right shifts carry an explicit
failure `ThrowKind`, validate both integer operands, execute width truncation
or signed/unsigned right extension, and reject an out-of-range distance through
that selected throw. Checked integer casts likewise carry their failure kind
and admit only values in the target signed/unsigned range. Move emits both
families with `abort`. Plain casts execute modular target-width conversion,
which gives Rust-style truncating/sign conversion a separate node. Plain shifts
are partial outside the fixed-width distance range; Rust imports preserve the
compiler-emitted overflow assertion before the shift, while checked forms keep
their explicit language-selected failure outcome. Fixed-width integer literals are
validated against the same shared signed/unsigned bounds before executable
preparation; pointer-width literals remain profile-dependent and unbounded
logical integers accept every value. Non-consuming local and place reads now
require the core `Copy` ability. Consequently `moveValue` is executable as an
identity over its already-evaluated operand: consuming a local is represented
by the place-based `move`, while a local expression cannot silently duplicate
a non-`Copy` value. Neither form is delegated to a Move string tag.
Value-level borrow, dereference, and freeze are a typed reference-operation
family. Under the prophetic ownership model
([`prophetic-references.md`](prophetic-references.md)) they execute over
borrow values: dereference observes the borrow's current value, mutable
update rewrites the live borrow at rest or reconciles a consumed temporary,
and freeze consumes the mutable loan into a shared snapshot. Value-level
borrow remains gated until checked place normalization rewrites eligible
inputs to the stronger existing borrow.
Field selection/update and variant selection/testing use typed data operations
with strong qualified references. They execute against resolved nominal
declarations, preserve the active enum variant during updates, and report a
dedicated source-located error when a runtime nominal shape does not match the
operation. Range construction, implication, equivalence, and identity are
shared primitive nodes.
Specification numbers use the core unbounded integer form; range, event-store,
type-domain, resource-domain, and state-domain types are typed core logical
nodes with strong child IDs. Resource-domain names resolve to nominal
declarations during verification preparation, including generic arity and
first-order ability checks on their type arguments.
Specification calls and built-ins use the closed, payload-bearing
`SpecOperation` family as well. Function targets are `QualifiedRef`s; result
indexes, state anchors, trace kinds, memory ranges, and integer widths are
typed fields. Move XAST and the Leaner frontend translate these operations in
both directions without routing known logical constructs through profile
strings. The Move profile consequently admits no residual type or operation
tags; its remaining tags describe frontend-only declaration metadata.
Remaining M2 work includes general index
evaluation, full type/ability/initialization checking, profile lifetime and
borrow constraints, and full preservation. The prophecy-reference refinement
obligation is gone: validated LIR runs one prophetic ownership semantics for
execution and verification alike
([`prophetic-references.md`](prophetic-references.md)).

The intended endpoint is:

> Validated LIR is the only semantic input to Leaner execution,
> verification, executable lowering, and generated Lean declarations.

Source provenance must survive every path out of LIR. Validation errors,
interpreter errors, language throws, verification obligations, and backend
diagnostics must point to the originating `LocId` and, through it, the
authored source range. A pass may refine or add related locations, but it may
not replace an available authored location with only a generated one.

The source language remains useful. Leaner source becomes one frontend which
constructs raw LIR, just as Move and Rust MIR do. What is retired is the
second, direct path that derives semantics by walking retained Lean syntax.

The first implementation target is an interpreter and a relational big-step
semantics for structured LIR. Verification is then defined against that
semantics. Existing source semantics and proof code may be copied into the
new package: the old implementation is a migration source, not a compatibility
boundary that must remain internally unchanged.

## Terminology

“Elaboration” currently conflates two operations which the new architecture
keeps separate:

1. **Frontend elaboration** parses and resolves a source language and creates
   a `RawUnit` with source locations and alignment evidence.
2. **LIR semantic elaboration** consumes a `ValidatedUnit` and registers Lean
   declarations for its runtime meaning, contracts, proof interface, and
   optional executable façade.

Only the second operation is shared by every frontend. Lean still elaborates
the syntax of a Leaner source file, but source syntax no longer defines the
function's semantic or verification path after LIR construction.

“Execution” below means execution of LIR itself. Lowering Move-profile LIR to
the existing stackless `MoveModel.IR` and then to bytecode remains a separate
backend and a separate preservation obligation.

## Current architecture

The current Leaner Move path derives related meanings independently:

```text
Leaner source
  |
  +--> Lean elaboration --> Lean definitions / Move.Action façade
  |                              |
  |                              +--> retained raw Lean Syntax
  |                                      |
  |                                      +--> Move.Verify.Source
  |                                           --> sourceSpec / contract / proof
  |
  +--> Lean environment / LCNF --> Move.Compiler.LIR (NSIR)
                                    --> MoveModel.IR --> interpreter / bytecode
  |
  +--> transitional Leaner-to-LIR importer
```

In particular:

- `move/Move/Verify/Syntax.lean` stores a private retained `Declaration`
  containing raw `Syntax`, reparses source, reconstructs signatures, analyzes
  borrow scopes, and generates relational specifications;
- `move/Move/Compiler/LIR.lean` independently recovers executable structure
  from elaborated Lean declarations;
- `move/Move/Compiler/Elab.lean` quotes the lowered `MoveModel.IR` back into
  Lean; and
- the current LIR path can print canonical Leaner source, but it does not yet
  supply the elaboration, execution, or proof meaning of that LIR.

This duplication is the problem being removed. It permits the compiler and
verifier to disagree, makes imported MIR a special case, and causes source
syntax and Lean environment side tables to remain semantic inputs long after
a validated LIR body exists.

## Target architecture

```text
Leaner source --------+
Move source ----------+--> RawUnit --> LeanerIR.Validation.validate --> ValidatedUnit
Rust MIR -------------+                                      |
                                                             |
                         +-----------------------------------+----------------+
                         |                                   |                |
                         v                                   v                v
                 LIR interpreter                    LIR big-step       LIR contracts
                 (computable)                       semantics          and WP
                         |                                   |                |
                         +---------- soundness --------------+------ verify f
                                                             |
                                      +----------------------+-------------+
                                      |                                    |
                                      v                                    v
                              executable backend                 Lean declaration UI
                              (Move IR/bytecode)                  and source backend
```

The arrows out of `ValidatedUnit` may reject a feature through an explicit
capability check. They may not consult the source AST or silently reinterpret
an unsupported node. Function `Origin` affects diagnostics and alignment, not
semantics.

### Package ownership

`leaner-ir/LeanerIR` owns:

- the runtime value and state model for the known Move/Rust semantic union;
- structured expression, place, call, control, and throw semantics;
- the executable interpreter;
- the authoritative relational big-step semantics;
- contracts, semantic weakest preconditions, calculated WP rules, and their
  soundness theorems;
- the generic `verify` proof interface over LIR functions; and
- semantic capability checking and source-positioned semantic diagnostics,
  including runtime termination sites and proof-obligation origins.

The package continues to depend only on Lean base libraries. It must not
depend on `move`, `move-model`, or `transpiler`.

The module and namespace ownership mirrors the pipeline:

- `LeanerIR.Import` owns raw frontend envelopes and checked CFG
  structurization;
- `LeanerIR.Validation` owns diagnostics, validated envelopes, structural
  checking, and semantic capability preparation;
- `LeanerIR.Semantics` modules own the runtime domain and big-step relations;
- `LeanerIR.Interpreter` owns executable evaluation; and
- `LeanerIR.Proofs` owns soundness and future verification theorems.

Test modules live below `LeanerIR.Tests` and are named for the boundary they
exercise; there is no undifferentiated `LeanerIR.Tests` source module.

Profile libraries supply implementations only for genuine extension nodes
and profile policies such as transaction-abort rollback or panic cleanup.
Known Move and Rust constructs belong in the core union. The Move profile no
longer admits residual type or operation tags; its remaining property tags
are frontend-only declaration metadata awaiting structural declaration
fields. A vocabulary check alone is not an executable meaning.

The other packages become clients:

- `move` owns Leaner surface syntax, the Leaner-to-LIR frontend, compatibility
  commands, and Move executable lowering;
- `leaner-move` supplies Move policy and validates residual frontend-only
  property metadata;
- `move-model` is a lower executable/prover IR and no longer defines LIR
  meaning; and
- `transpiler` owns exchange adapters and source printing, not semantics.

## Semantic preparation boundary

`ValidatedUnit` currently guarantees structural properties such as ID bounds,
arena acyclicity, profile vocabulary, and structured control. It does not yet
guarantee every property required for safe evaluation. Before the interpreter
or verifier becomes public, validation must additionally establish:

- resolved declaration and call targets;
- expression, pattern, place, and operation typing;
- generic argument and ability satisfaction;
- local scope, initialization, move, and drop legality;
- call and return arity;
- valid `break_` and `continue_` nesting;
- constructor, destructor, and variant consistency;
- lifetime and borrow constraints required by the selected profile;
- contract and specification-expression typing; and
- a semantic implementation for every reachable extension node.

Backend-specific feature selection remains separate. Proposed wrappers with
private constructors make accidental partial execution impossible:

```lean
prepareExecution  : SemanticsRegistry -> ValidatedUnit ->
  Except (Array Diagnostic) ExecutableUnit

prepareVerification : SemanticsRegistry -> ValidatedUnit ->
  Except (Array Diagnostic) VerifiableUnit
```

These wrappers contain indexes and checked handlers; they do not copy or
translate function bodies. Eventually the common checks should move into
`LeanerIR.Validation.validate`, leaving preparation to check only consumer
capability.

## Runtime model

### Values

The initial interpreter should use a first-order `RuntimeValue` plus a
separate typing judgment, rather than a dependent value indexed by an arena
`TypeId`. Arena-indexed dependent values would make execution and decoding
needlessly difficult while the schema is still evolving.

The runtime value union needs first-class cases for:

- unit, booleans, integers, strings, and bytes;
- tuples and vectors, including fixed-length vector validation;
- nominal structures and enum variants;
- closures with captured values;
- references to runtime locations; and
- checked extension values whose semantics handler owns their representation.

`ValueHasType unit namespace value typeId` states runtime well-typedness.
Preservation later proves that evaluation of a semantically checked function
cannot produce a value outside its declared result types.

### State and places

Runtime state contains:

- a call stack with initialized/uninitialized locals;
- global/resource storage;
- the pending write-back set of loans exported by dying frames; and
- any profile-owned transaction state.

There is no heap and there are no reference targets: under the prophetic
ownership model ([`prophetic-references.md`](prophetic-references.md)) a
borrow is a value owning the loaned content, a mutable loan leaves a hole
where the value was taken, and a resolved place is a root plus a projection
path whose dereference step projects into the borrow's current value.

Keyed global storage uses the core `GlobalKind` operations `contains`,
`borrow`, `take`, and `publish`. A mutable global borrow leaves its hole in
the global slot; take and publish interact with holes only through certified
exclusivity. Move `existsAt`/`borrowGlobal`/`moveFrom`/`moveTo` map directly
to these nodes instead of profile-operation tags.

Lifetimes are not runtime values. They are checked evidence consumed by the
borrow analysis, whose certificate licenses the prophetic model. The
interpreter and the big-step relation run the same deterministic rules —
loan-death markers write borrows back, dying frames export through the
pending set — so there is no separate prophecy judgment to relate: the
prophecy is the contract-level reading of the pending exports.

Move abort and Rust panic must remain distinct `ThrowKind`s. Evaluation first
produces a thrown outcome with its argument list. A profile-specific
invocation boundary then decides state visibility:

- Move abort exposes the pre-transaction global state;
- Rust panic preserves mutations or runs cleanup according to the selected
  panic policy; and
- interpreter exhaustion and malformed state are tool errors, never language
  throws.

The semantic outcome remains independent of provenance, but executable
results pair it with an `ExecutionOrigin`. At minimum this records the
`LocId` of the expression that returned or threw. Runtime errors record the
location of the operation which became stuck or exhausted its local fuel
budget. Calls retain both the call-site location and the callee location as a
compact stack so an abort or panic can be reported at its origin with related
caller ranges. Locations are observational metadata: changing them cannot
change whether a program returns, throws, or mutates state.

## Structured big-step semantics

The authoritative semantics is an inductive relation over a structured body,
not a translation to CFG or `MoveModel.IR`:

```lean
inductive Control where
  | value (values : Array RuntimeValue)
  | break_ (nest : Nat) (value : Option RuntimeValue)
  | continue_ (nest : Nat)
  | return_ (values : Array RuntimeValue)
  | throw_ (kind : ThrowKind) (arguments : Array RuntimeValue)

EvalExpr : SemanticContext -> FrameState -> ExprId ->
  FrameState -> Control -> Prop
```

The exact signatures may evolve, but the following properties are required:

- a block propagates non-local control without evaluating its suffix;
- `loop` consumes a matching depth-zero break/continue and decrements outer
  nesting depths when propagating them;
- a call evaluates arguments in LIR-defined order, enters a fresh frame, and
  converts a callee return into a caller value;
- `throw_` preserves all argument values;
- patterns either bind atomically or do not match;
- place assignment evaluates its right-hand side before mutating its target;
- closure packing captures values in the declared order and `invoke` calls
  the captured function value;
- specification blocks have no runtime effect; and
- absent bodies can execute only through a registered external summary or
  implementation.

Nontermination is represented by the absence of a finite big-step derivation.
The semantics contains no fuel and no `outOfFuel` outcome.

Function meaning is a small wrapper around this relation:

```lean
structure FunctionMeaning where
  relates : Invocation -> RuntimeState -> RuntimeState -> Outcome -> Prop
```

It is selected by profile and function identity, never by whether the body
came from Leaner, Move, MIR, or generated source.

**Open rules (2026-09-03).** The rules are stated over a callee oracle:
`EvalExprWith unit callee` (with `EvalValuesWith`, `EvalStatementsWith`,
`EvalArmsWith`) answers every direct and closure call by
`callee : CalleeRelation`, and the closed semantics `EvalFunction unit` is
the nested inductive tying the knot — its one rule evaluates the body under
`EvalFunction unit` itself.  `EvalExpr unit` and the other closed relations
are the instances at that oracle, so every consumer keeps its spelling.
The knot's metatheory lives in `LeanerIR/Proofs/Oracle.lean`: the open
rules are monotone in the oracle, and `EvalFunction.induction` states that
the closed semantics is below every oracle closed under one unfolding of
the function boundary — the least-fixed-point induction a recursive
function's native denotation is proved against
([`certifying-execution.md`](certifying-execution.md), *Recursion*).

## Interpreter

The interpreter mirrors the big-step rules but is explicitly fuelled over
loops, recursion, and calls:

```lean
interpret : ExecutableUnit -> Fuel -> FunctionRef ->
  RuntimeState -> Array RuntimeValue ->
  Except LocatedInterpError (RuntimeState × LocatedOutcome)
```

`LocatedInterpError` contains an `InterpError`, a primary `LocId`, and related
call locations. `InterpError` distinguishes at least unsupported external
implementation, stuck/malformed runtime input, and `outOfFuel`.
`LocatedOutcome` contains the semantic `Outcome` plus its `ExecutionOrigin`.
A language `throw_` is therefore a successful, source-positioned interpreter
result.

The initial implementation should copy the useful finite-store and proof
patterns from `MoveModel.IR.Interp`, but it must execute structured LIR
directly. Defining LIR execution by lowering to `MoveModel.IR` would make the
lowering correct by definition, lose the structured semantics wanted for
Lean and Rust, and prevent that lowering from being tested independently.

Required metatheory, in implementation order:

1. lookup/update lemmas for finite runtime stores;
2. operation-level interpreter soundness;
3. expression and control-flow interpreter soundness;
4. function/call interpreter soundness;
5. determinism of the big-step semantics for executable profiles;
6. completeness up to fuel: every finite derivation is reproduced with
   sufficient fuel; and
7. type preservation and a no-stuck theorem for prepared invocations.

The foundational theorem is:

```text
interpret executable fuel f state args = ok (state', outcome)
  -> f.semantics.Relates state args state' outcome
```

`outOfFuel` makes no semantic claim and is never converted to abort or panic.

## Contracts and verification

### One meaning, two useful representations

Verification targets `FunctionMeaning`, not the interpreter implementation.
The simplest semantic WP is initially defined from the big-step relation:

```text
SemanticWP f normalPost throwPost initialState :=
  every outcome related by f.semantics satisfies the matching postcondition
```

A calculated `wpExpr` is then implemented by structural recursion over LIR so
proofs do not quantify over derivation trees. Its soundness theorem connects
it to `SemanticWP`. This ordering prevents tactics from becoming the
definition of correctness.

The current relational machinery in `Move/Semantics` and `Move/Verify` should
be moved or copied in layers:

- common outcomes, bind, contracts, and fixed points first;
- checked scalar/vector operations and calls;
- concrete global effects and profile finalization;
- prophecy-backed immutable and mutable borrow rules;
- loop and recursion induction;
- frames, invariants, and modular summaries; and
- proof automation last.

> Superseded 2026-08-27: the runtime and proof representations of
> references ARE identical. Validated LIR is prophecy-based ownership per
> [`prophetic-references.md`](prophetic-references.md); the interpreter
> runs the ghost-erased rules, so the former concrete-to-relational
> refinement obligation is replaced by that design's agreement theorem.

### Specification expressions and contracts

Contracts already live in LIR as `FunctionContract`, `Condition`, `Frame`,
and expression roots. The verifier interprets these nodes directly:

- `requires` constrains the initial logical environment;
- `ensures` observes results, final referents, and final state;
- exceptional clauses match `ThrowKind` and all throw arguments;
- `old` selects the initial state anchor;
- frame clauses constrain unmentioned state; and
- loop invariants attach to their structured loop point rather than a
  reconstructed source loop.

Logical quantifiers and uninterpreted specification functions have relational
denotations even when they are not executable. Therefore preparation for
verification can accept a larger subset than preparation for execution, but
must diagnose any construct for which no logical denotation exists.

### Generated proof interface

For each executable declaration `f`, LIR semantic elaboration exposes the
equivalent of:

```lean
f.lirDecl    -- stable handle into the validated unit
f.semantics  -- FunctionMeaning
f.spec       -- denotation of the LIR FunctionContract
f.contract   -- Contract.Satisfies f.semantics f.spec
f.verified   -- theorem produced by `verify f`
```

During migration, `f.sourceSpec` may remain as a compatibility alias for
`f.semantics`; it must not be generated by walking retained source. Calls use
proved or trusted summaries attached to LIR function identities, so callers
and callees may come from different frontends when their profiles are
compatible.

`verify f` becomes a small command elaborator which resolves `f.lirDecl`,
constructs the standard `Contract.Satisfies` theorem, and invokes LIR proof
automation. It does not retrieve raw source syntax.

## Lean declaration elaboration

LIR semantic elaboration should quote or register each validated unit once.
Generated declarations refer to a unit constant and stable declaration IDs;
they should not expand every expression node into a fresh Lean syntax tree.
This avoids repeating an arena once for execution, again for verification,
and again for compilation.

The elaborator has two outputs:

1. **Semantic declarations**, such as `f.semantics`, `f.spec`, and stable LIR
   handles. These are required for every supported function.
2. **Ergonomic façades**, such as a typed Lean function or `Move.Action`
   wrapper. These are generated only when the function's profile and types
   can be represented by that surface API.

Nominal LIR declarations similarly generate Lean structures/enums and the
type-reification evidence needed to cross between typed façades and
`RuntimeValue`. Generic bodies stay generic over type, const, lifetime, and
trait/ability evidence; the semantic elaborator must not monomorphize them
merely to produce a Lean declaration.

To control elaboration size:

- quote a `ValidatedUnit` once and use declaration indexes;
- retain arena sharing when calculating WP terms;
- make common semantic combinators ordinary definitions and lemmas;
- generate small wrappers rather than one fully expanded term per consumer;
- memoize per-expression verification results where it materially reduces
  repeated traversal; and
- keep source/provenance tables out of definitional equality and proof terms.

### Diagnostics and source mapping

Every generated declaration and obligation records its originating
`NamespaceId`, declaration ID, and `LocId` in a persistent environment
extension. Errors from semantic preparation use the LIR location directly.
Errors raised while elaborating a generated Lean declaration are translated
through this reverse map before presentation.

Generated subterms inherit the most specific LIR node location. A diagnostic
may add related source ranges for the call site, callee declaration, macro
origin, or imported MIR location. The canonical Leaner printer is another
consumer of this map; generated text is not the authoritative error location.

Location propagation is required at every semantic boundary:

- validation reports the invalid expression, place, declaration, condition,
  or frame location rather than only the enclosing namespace;
- the interpreter reports a `throw_` at the throwing expression and a stuck
  operation at the operation expression, with caller locations as related
  ranges;
- calculated WP nodes and generated Lean goals retain the `LocId` of the LIR
  code or contract clause which created them;
- a malformed specification points to its `Condition`, `Frame`, or
  specification-expression location;
- an unproved verification goal is displayed at the relevant code or spec
  location, even when the generated theorem has synthetic Lean syntax; and
- executable and source lowerings maintain an explicit target-node-to-LIR-
  location map so later backend errors can be translated back through LIR.

When one target node combines several LIR nodes, it keeps one primary
location and all other contributing locations as related ranges. When one LIR
node expands to several target nodes, every target inherits its location.
Generated nodes without a direct authored range retain `generatedBy` and
`parent` links until an authored ancestor is reached.

## Migration plan

Each milestone should be independently reviewable and leave both paths
working until its gate is met.

### M0 — Inventory and semantic capability gate (implemented)

- Inventory every core node and every current Move `ProfileValue` tag as
  executable, logical-only, frontend-only, or unsupported.
- Extend validation enough to type the first executable subset.
- Add `SemanticsRegistry`, `ExecutableUnit`, and `VerifiableUnit` preparation
  boundaries.
- Add a CI inventory check so a new syntax node cannot silently lack semantic
  classification.

Gate: a validated fixture either prepares successfully or reports a stable
source-positioned capability diagnostic for every reachable node.

### M1 — Values, state, big-step core, and interpreter (implemented)

- Add runtime values, frames, stores, outcomes, and type judgments.
- Implement big-step and fuelled interpreter rules for constants, locals,
  blocks, lets, patterns, assignment, `ifElse`, `match_`, return, throw, and
  direct calls.
- Add structured `loop`, `break_`, and `continue_` without lowering to CFG.
- Propagate expression locations into returned/thrown outcomes, interpreter
  failures, and call stacks.
- Prove operation and interpreter soundness for this subset.

Gate: hand-built validated LIR runs scalar, tuple, branch, loop, call, normal
return, and throw examples; thrown and stuck cases identify the expected
source range; and every interpreter test also has a big-step soundness
instance.

### M2 — Data, operations, references, and closures

Nominal construction/destruction, field update/selection, enum variant tests,
variant-field selection, vectors, checked arithmetic, shifts, and casts, fixed-width bitwise
operations, closures, concrete references, keyed globals, and throw
finalization now have executable rules and interpreter-soundness coverage. The
fixed-width signed division and remainder rules round toward zero, and checked
remainder rejects the signed `MIN % -1` quotient overflow. The
concrete reference rules include value-level dereference, freeze, and mutation.
Validation also normalizes value-level borrows of direct local expressions to
the existing place-based borrow. Identical local places are reused and missing
ones are appended, so authored IDs stay stable without duplicate place nodes;
non-place inputs remain explicitly unsupported. The closed
primitive vocabulary also has source-located operand/result type checks.
Raw validation and semantic preparation share one first-slice literal/type
matcher, preventing their constant rules from drifting as generic declaration
checks grow. Top-level constant initializers are prepared under their owning
namespace profile rather than an implicit Move context.
The remaining work is the unsupported primitive/reference subset,
trait/predicate satisfaction, non-nominal
whole-unit type well-formedness, the remaining declaration families, plus
preservation and no-stuck proofs.

Trait-associated method signatures use the same owner-prefix scoping in raw
validation and semantic preparation: owner trait binders precede method-local
binders for nested type and ability checks.

- Add nominal construction/destruction, enums, vectors, checked arithmetic,
  and fixed-length vectors.
- Implement places, concrete references, reborrows, calls with references,
  and returned references.
- Add closure packing and invocation.
- Add global/resource and heap behavior plus profile throw finalization.
- Prove preservation and interpreter/reference agreement for the supported
  profiles.

Gate: the interpreter covers the executable portion of the existing Leaner
language corpus without routing through `MoveModel.IR`.

### M3 — Shadow semantic elaboration for Leaner source

> Reframed 2026-08-27: with the direct path deprecated rather than live, the
> shadow/differential arrangement is no longer the goal; what survives is
> registering complete raw LIR units from the LeanerLang frontend, which is in
> place. The differential-execution role passed to the MonoVM harness.

- Make the Leaner frontend register its complete raw LIR unit.
- Run LIR semantic elaboration beside the current direct definitions.
- Generate `f.lirDecl`, `f.semantics`, and execution wrappers from LIR.
- Make `lowerToIR` consume the registered validated LIR rather than rediscover
  the body from Lean LCNF.
- Differentially execute the old compiled path and the new LIR interpreter.

Gate: supported source programs have equal observable return/throw and state
results on both paths, with no source-syntax fallback inside the LIR path.

### M4 — Contracts and calculated WP

- Interpret LIR specification functions, conditions, frames, and invariants.
- Copy the common contract and semantic-WP foundation into `LeanerIR.Proofs`.
- Implement and prove sound the calculated WP for the M1 executable subset.
- Generate `f.spec` and `f.contract` from LIR.
- Switch `verify f` to LIR lookup for migrated functions.
- Attach the originating code or specification `LocId` to every generated
  obligation and translate unsolved goals back to source.

> Status 2026-08-27. Landed in `LeanerIR/Proofs`: `Spec`/`Contract` (the
> reference stack's relational model and contract calculus generalized over
> the failure vocabulary `ε`; simp sets `lir_wp_norm`/`lir_spec_norm` —
> prefixed because the frozen stack owns the unprefixed names in shared
> import closures), `Meaning` (`functionSpec` reads the relational meaning
> off the big-step judgment, with two-way interpreter agreement and the
> `satisfiesFunction_of_wp` entry point), and `WP` (demonic transformers
> over the five big-step judgments, the `wp_functionSpec`/`wpFunction_body`
> bridges, and calculation rules for every expression kind, with the
> unconditional step equations and stepping tactics that symbolically
> execute a quoted unit).
>
> Status 2026-08-29. The M4 gate is met. Generated contracts read the full
> Move abort discipline (`aborts_if … with code` pins the failure outcome;
> `aborts_if_is_partial`/`aborts_if_is_strict` select the reference stack's
> `abortComponents` readings), named constants and `spec.bitVectorToInt`
> translate in clauses, and every generated obligation carries the authored
> clause's byte range through an `Obligation` marker — a residual goal is
> reported by `leaner_report` as an error at that clause, which is the
> LocId-attachment bullet delivered for LeanerLang-authored units. The
> first verification tests copied from the reference stack
> (`LeanerLang/Tests/VerificationAborts.lean`, from `AbortDirections`)
> prove the same public contracts from LIR, and a deliberately wrong body
> fails at the expected `ensures` range under `#guard_msgs`. The prepared
> unit is quoted once per namespace (`<ns>.semantics` with kernel-checked
> equations), so every verify in a namespace shares one preparation and
> goals reference the unit by name.
>
> Alignment with the reference `verify` pipeline (`Move.Verify.Tactics`,
> `Move.Verify.Syntax`): the LIR analog of `f.sourceSpec` is the uniform
> `functionSpec` — no per-function derivation from retained source. Because
> LIR is a deep embedding, symbolic execution needs the arena lookups
> reduced; the generator therefore quotes the validated unit's per-function
> data as literals and derives each function's shallow WP unfolding through
> the calculation rules once, after which `verify f` behaves as in the
> reference stack: contract intro → calculated-WP normalization → case
> splits, callee contracts, loop invariants → arithmetic finish. The
> unbounded-integer value domain replaces the reference stack's
> width-indexed `UInt` normalization inventory; range facts come from
> validation typing (`IntegerValueFits`), which keeps the arithmetic
> finish omega-shaped. Manual proofs keep the reference surface: one
> goal-shaped step tactic, checked-operation case splits, contract-backed
> call stepping, declared-abort clause discharge.

Gate: the first copied verification tests prove the same public contracts
from LIR, and a deliberately wrong body fails against the unchanged contract
at the expected body or specification range.

### M5 — Borrow, state, loop, and modular verification

- Port prophecy-backed references and their concrete-execution refinement.
- Port checked operations, rollback, globals, frames, calls, summaries,
  recursion, loops, and invariants.
- Ensure calls consume LIR summaries without inspecting callee source.
- Carry all trusted external summaries and frontend alignment assumptions into
  theorem reports.

Gate: the representative reference, global-state, loop, and cross-call proof
tests pass solely through LIR semantics.

> Status 2026-08-29 (second revision). Global state verifies through a
> **typed spec-level state**, replacing both the untyped accessor reading and
> the earlier state-typing assumption. Per struct declaration the frontend
> generates a typed twin (certified `SpecInt` integers, plain Lean scalars,
> nested twins) with an `erase`/`decode?` pair and proved roundtrips
> (`LeanerLang/SpecTypes.lean`); a contract's `requires` quantifies one
> typed map `StorageKey → Option Twin` per storable family, tied to runtime
> memory by `FamilyRepresentation` — the map laws are proved on the keyed
> `GlobalMap` and cross-family disjointness is a theorem of key
> disequality, not an assumption. Clauses read storage through the typed
> accessors, so a resource read out of memory is the erasure of a typed
> value: constructor shape by reduction, range facts from the certified
> fields. `leaner_storage` routes a stuck lookup onto the typed map through
> the representation hypothesis and case splits the typed entry;
> `leaner_drive` replaces the stock repeat combinators with an explicit
> once-per-goal fixpoint driver. `modifies global<T>(k)` generates the
> keyed frame (every other key reads the same), discharged by the map laws.
>
> Landed and verified automatically: reads (`balance_of` shapes, existence,
> boolean fields, `spec.old`), `move_from` with post-state clauses and
> frames, and whole-resource `&mut` update (`*coin := new Coin {...}`).
> The write-back of a mutable global borrow is keyed by the state's loan
> registry (`RuntimeState.globalLoans`) — certified exclusivity makes the
> recorded key the hole's location, so nothing searches global memory.
> Signers key storage by their address in `RuntimeValue.storageKey?`.
>
> Known boundary: field projection *through* a mutable global borrow
> (`&mut Coin[addr].value`) lowers to a data `select` on a borrow value,
> which the core evaluator does not define — the pattern was never
> executable, independently of verification. Until the lowering emits the
> place-based pattern locals use (or the core defines focused reference
> projection), the supported surface form is the whole-resource borrow.
> Above the gate sit calls through callee contracts and loops through
> invariants.

### M6 — Corpus migration and default-path switch

> Reframed 2026-08-27: there is no default-path switch to make; the old
> corpus is an oracle to copy from (see Test migration), and the deprecated
> packages are excluded from test runs.

- Move the Leaner test corpus in feature waves, maintaining an explicit
  supported/unsupported ledger.
- Change Leaner commands so the LIR path is the default and the old path is
  available only behind a temporary comparison option.
- Replace compatibility `sourceSpec` declarations with aliases.
- Prevent new tests from using the direct elaborator.

Gate: every existing test is migrated, deliberately retired, or has a tracked
unsupported diagnostic; the normal build does not read retained raw source to
derive semantics or proofs.

### M7 — Retire the direct path

> Reframed 2026-08-27: retirement happened by wholesale deprecation instead
> of incremental deletion; the packages remain in-tree as frozen reference.

- Delete the retained-source `Declaration` store and source reparser.
- Delete duplicated source signature, source borrow-scope, and source-to-Spec
  translation code after their last LIR equivalent lands.
- Remove the environment/LCNF body rediscovery path from executable lowering.
- Keep surface syntax, compatibility names, and proof tactics only where they
  are thin clients of LIR.

Gate: searching the production path finds no semantic analysis over retained
Lean syntax, all frontends meet at `ValidatedUnit`, and execution,
verification, and lowering consume the same function body.

## Test migration

Tests should be copied incrementally rather than moved wholesale. The old
tests remain an oracle until their feature wave converges. Each copied case
records four observations where applicable: return values, throw kind and
arguments, visible final state, and generated contract result. Throw and
failure cases also assert their primary and relevant related source ranges.
Fuel is a test harness parameter and is never part of expected language
behavior.

Suggested waves are:

1. literals, integer arithmetic, tuples, blocks, conditionals, and ordinary
   returns;
2. loops and structured control exits;
3. structures, enums, vectors, generics, direct calls, and closures;
4. immutable and mutable local references, nested places, and returned
   references;
5. global resources, rollback, frames, and cross-module calls;
6. basic contracts and abort directions;
7. borrow, loop, recursion, invariant, and modular-summary proofs; and
8. negative surface, type, borrow, lowering, and verification diagnostics.

There are three complementary test forms:

- **semantic unit tests** construct small raw LIR fixtures, validate them, and
  run the interpreter;
- **frontend integration tests** elaborate Leaner source to LIR and run that
  validated artifact; and
- **differential tests** compare LIR execution with the existing
  `MoveModel.IR` interpreter or transactional VM for the common subset.

Big-step tests should not merely recompute the interpreter. Successful
interpreter evaluations use the soundness theorem to produce semantic facts;
selected examples also construct or invert relational derivations directly.
Negative tests assert diagnostic codes and source ranges before execution.

The aggregate test roots should be split by concern, for example:

```text
LeanerIR/Tests/Validation.lean
LeanerIR/Tests/Interpreter.lean
LeanerIR/Tests/Semantics/*
LeanerIR/Tests/Verification/*
LeanerIR/Tests/Elaboration/*
```

This keeps the independent `leaner-ir` test suite useful without importing
the old `Move` package. Cross-package frontend agreement and VM differential
tests live in the dedicated `leaner-e2e-tests` package rather than adding
frontend dependencies to `leaner-ir` or a semantic-profile package.

## Correctness obligations

The migration is complete only when the following claims are explicit:

- interpreter results are admitted by LIR big-step semantics
  (`run_sound`, implemented);
- finite deterministic LIR executions are reproduced with sufficient fuel
  (`evalFunction_complete`/`run_complete` over fuel monotonicity, implemented;
  big-step determinism of `EvalFunction` and `FunctionMeaning` follows
  because the interpreter is a function);
- semantic preparation plus well-typed inputs implies preservation and no
  stuck execution;
- interpreter outcomes, errors, and generated proof obligations preserve
  their originating LIR locations;
- calculated WP implies semantic WP;
- `f.verified` proves `Contract.Satisfies f.semantics f.spec`;
- concrete reference execution refines the prophecy verification model;
- Move abort finalization rolls back exactly the required state;
- each executable lowering preserves LIR outcomes;
- each frontend's alignment evidence justifies relating its source artifact
  to LIR semantics; and
- canonical source re-elaboration preserves normalized LIR semantics.

The first six are owned by `leaner-ir`.

### Preservation and no-stuck decomposition

The remaining M2 metatheory is staged as follows; `ValueHasType` from M1 is
the structural layer it builds on.

1. **Store typing.** A `StateTyping` assigns an optional `TypeId` to every
   heap slot; global slots carry their resource `TypeId` already. A typed
   value judgment strengthens `ValueHasType` at references: the target slot's
   assigned root type, projected through the reference's projections, is the
   reference type's referent. Nominal field and closure capture typing deepen
   the M1-shallow cases and need the owning `ExecutableUnit`, not just
   `Tables`; they join the judgment when the field/capture preservation
   lemmas land.
2. **Frame and state judgments.** `TypedFrame` relates a declaration's local
   table to a runtime frame: initialized locals are typed at their declared
   types, uninitialized locals are `none`. `TypedState` requires every live
   heap and global slot to be typed at its assigned type.
3. **Leaf lemmas.** Validation's literal/type checker
   (`firstSliceConstMatchesType`) and the reifier (`constValue?`) agree with
   `ValueHasType`; `initialFrame?` produces a typed frame from typed
   arguments. Both functions must be total for these lemmas to exist, so the
   nested `ConstValue` recursion is structural, not `partial`.
4. **Operation lemmas.** Every semantic operation evaluator preserves typing
   and — for no-stuck — is defined on the operand shapes semantic
   preparation admits. Each `LIR-SEMANTIC-TYPE` rule in validation names the
   shapes its operation lemma may assume.
5. **Evaluator induction.** Preservation threads a growing `StateTyping`
   through the mutual evaluators; no-stuck additionally consumes the
   initialization certificate carried by `ValidatedUnit` (reads are
   initialized on every path the certificate covers) and concludes that a
   prepared, typed invocation returns a result or `outOfFuel` — never a
   stuck diagnostic.

> Status 2026-08-27: stages 1–4 are proved in
> `LeanerIR/Semantics/Typing.lean` for the pure primitive vocabulary —
> `WfPrimitive.eval_typed` covers the scalar, boolean, character-cast, and
> overflowing groups (bitwise operations under an integer result node need
> the operand-scalarity premise), and per-operation lemmas cover the
> aggregate group plus copy/move forwarding. Deferred: pointer-width
> integer nodes (`.integer .pointer` resolves against a target before
> `ValueHasType` can state the node), deep nominal-field and
> closure-capture typing, and the stage-5 induction itself.

Profile finalization laws are proved
against interfaces declared there. Lowering and frontend alignment theorems
are owned by their adapters, then composed with LIR verification. Constructing
a `ValidatedUnit` alone does not prove source or MIR alignment.

## Compatibility policy

The migration should preserve user-facing source names and the shape of
`spec f` / `verify f` where practical. Internal generated names and theorem
statements may change when needed to state the LIR result honestly.

Compatibility aliases are acceptable only when they point from the old API to
new LIR-owned declarations. New LIR semantics must never call back into the
old source translator. A temporary differential mode may run both paths and
report disagreements, but neither result may silently replace the other.

Unsupported constructs fail during validation or semantic preparation with a
location and stable diagnostic code. Falling back to source elaboration would
hide coverage gaps and is therefore forbidden once a function opts into the
LIR path.

## Settled decisions

- Validated structured LIR, not retained source syntax or lower CFG IR, is the
  semantic source of truth.
- The interpreter executes structured LIR directly and is fuelled only as a
  termination device.
- The relational big-step semantics is authoritative and contains no fuel.
- Verification is stated over big-step function meaning, with a proved-sound
  calculated WP for automation.
- Concrete runtime references and prophecy proof references may differ, but
  require a refinement theorem.
- LIR semantic declarations are independent of frontend origin.
- Source locations propagate through execution, verification, and lowering;
  they are diagnostic metadata and never alter semantic outcomes.
- Generic LIR bodies remain generic over evidence rather than being
  monomorphized for elaboration.
- Generated Lean declarations are small indexed façades over one registered
  unit, not repeated expansions of the LIR body.
- Existing elaboration and proof code may be copied into `leaner-ir` and then
  adapted structurally; keeping the old implementation DRY is not a goal.
- The direct source-semantic path is deleted only after corpus coverage and
  comparison gates make the LIR path the default.

## Open implementation choices

The following choices should be settled during M0 without changing the
architecture:

- the concrete finite-map representation used by executable global and heap
  stores;
- the exact typed façade generated for pure and effectful functions;
- whether semantic preparation wrappers are separate types or private
  capability records inside `ValidatedUnit` indexes;
- the stable identity and versioning scheme for registered semantic handlers;
- how much calculated WP is materialized eagerly versus memoized on demand;
  and
- the temporary compatibility name for `f.sourceSpec` before it becomes
  `f.semantics`.
