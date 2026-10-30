# Verex: Elaboration, Verification, and Execution from VIR

## Scope

This document defines the runtime model, the structured big-step semantics,
the interpreter, and the correctness obligations of validated Verex IR. It
builds on the shared representation in [`vir-design.md`](vir-design.md); it
does not redefine the VIR syntax or the frontend interchange format.
Verification over the semantics is the subject of
[`denotation.md`](denotation.md). Open work:
[`roadmap.md`](../plan/roadmap.md).

The principle is:

> Validated VIR is the only semantic input to Verex execution,
> verification, executable lowering, and generated Lean declarations.

Source provenance survives every path out of VIR. Validation errors,
interpreter errors, language throws, verification obligations, and backend
diagnostics point to the originating `LocId` and, through it, the authored
source range. A pass may refine or add related locations, but it may not
replace an available authored location with only a generated one.

## Terminology

“Elaboration” names two operations which the architecture keeps separate:

1. **Frontend elaboration** parses and resolves a source language and creates
   a `RawUnit` with source locations and alignment evidence.
2. **VIR semantic elaboration** consumes a `ValidatedUnit` and registers Lean
   declarations for its runtime meaning, contracts, proof interface, and
   optional executable façade.

Only the second operation is shared by every frontend. Lean elaborates the
syntax of a Verex source file, but source syntax does not define the
function's semantic or verification path after VIR construction.

“Execution” below means execution of VIR itself. Lowering Move-profile VIR to
bytecode is a separate backend and a separate preservation obligation.

## Architecture

```text
Verex source --------+
Move source ----------+--> RawUnit --> VerexIR.Validation.validate --> ValidatedUnit
Rust MIR -------------+                                      |
                                                             |
                         +-----------------------------------+----------------+
                         |                                   |                |
                         v                                   v                v
                 VIR interpreter                    VIR big-step       VIR contracts
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

`verex-ir/VerexIR` owns the runtime model, the structured semantics, the
interpreter, the relational big-step semantics, contracts and their
verification interface, and source-positioned semantic diagnostics; it
depends only on Lean base libraries. Its namespaces mirror the pipeline:
`Import` (raw envelopes, structurization), `Validation` (diagnostics,
checking, capability preparation), `Semantics` (runtime domain, big-step
relations), `Interpreter`, `Proofs`. Profile libraries supply
implementations only for genuine extension nodes and profile policies
(transaction-abort rollback, panic cleanup); known Move and Rust constructs
belong in the core union, and a vocabulary check alone is not an executable
meaning. `verex-move` and `verex-rust` are clients supplying their profiles;
neither defines VIR meaning.

## Semantic preparation boundary

`validate` establishes the structural properties of a `ValidatedUnit` (ID
bounds, arena acyclicity, profile vocabulary, structured control) and the
semantic properties below; it owns typing and the initialization and borrow
analyses, whose certificates travel on the `ValidatedUnit`:

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

Backend-specific feature selection remains separate. Wrappers with private
constructors make accidental partial execution impossible:

```lean
prepareExecution  : SemanticsRegistry -> (unit : ValidatedUnit) ->
  Except (Array Diagnostic) (ExecutableUnit unit)

prepareVerification : SemanticsRegistry -> ValidatedUnit ->
  Except (Array Diagnostic) VerifiableUnit
```

These wrappers contain indexes and checked handlers; they do not copy or
translate function bodies. Preparation checks consumer capability only.

## Runtime model

### Values

The interpreter uses a first-order `RuntimeValue` plus a separate typing
judgment, rather than a dependent value indexed by an arena `TypeId`.

The runtime value union has first-class cases for:

- unit, booleans, integers, strings, and bytes;
- tuples and vectors, including fixed-length vector validation;
- nominal structures and enum variants;
- closures with captured values;
- references to runtime locations; and
- checked extension values whose semantics handler owns their representation.

`ValueHasType unit namespace value typeId` states runtime well-typedness.
Preservation proves that evaluation of a semantically checked function
cannot produce a value outside its declared result types. The judgment uses
the same representable-range predicate for runtime integers as validation,
so an out-of-range fixed-width value cannot be used as a preservation
witness merely because its type-table entry is integer.

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
to these nodes.

Lifetimes are not runtime values. They are checked evidence consumed by the
borrow analysis, whose certificate licenses the prophetic model. The
interpreter and the big-step relation run the same deterministic rules —
loan-death markers write borrows back, dying frames export through the
pending set — so there is no separate prophecy judgment to relate: the
prophecy is the contract-level reading of the pending exports.

Move abort and Rust panic are distinct `ThrowKind`s. Evaluation first
produces a thrown outcome with its argument list. A profile-specific
invocation boundary then decides state visibility:

- Move abort exposes the pre-transaction global state;
- Rust panic preserves mutations or runs cleanup according to the selected
  panic policy; and
- interpreter exhaustion and malformed state are tool errors, never language
  throws.

The semantic outcome is independent of provenance, but executable results
pair it with an `ExecutionOrigin`. At minimum this records the `LocId` of
the expression that returned or threw. Runtime errors record the location of
the operation which became stuck or exhausted its local fuel budget. Calls
retain both the call-site location and the callee location as a compact
stack so an abort or panic can be reported at its origin with related caller
ranges. Locations are observational metadata: changing them cannot change
whether a program returns, throws, or mutates state.

### Primitive operations

Fixed-width arithmetic has modular operations, `checked*` operations
carrying their overflow `ThrowKind`, and overflowing operations returning
the modular value plus a flag; Move emits checked arithmetic with `abort`.
Signed division and remainder round toward zero; checked remainder rejects
`MIN % -1`. Bitwise OR, AND, XOR and complement normalize operands to the
result width and reconstruct signed results from the two's-complement
pattern. Checked shifts and checked casts carry their failure kind: a shift
rejects an out-of-range distance, a cast admits only values in the target
range. Plain casts convert modularly (Rust's truncating conversion), and
plain shifts are partial outside the distance range, Rust imports keeping
the compiler-emitted overflow assertion before them. Fixed-width literals
are validated against the shared bounds; unbounded logical integers accept
every value.

Fixed-width to logical-integer coercions are unary and check their
numeric domains. They are not Move's `int2bv`/`bv2int` representation
wrappers, which the Move importer rejects: erasing them would allow false
overflow proofs ([`numeric-representation.md`](numeric-representation.md)).

Non-consuming local and place reads require `Copy`; consuming a local is the
place-based `move`, and validation turns a direct read of a non-`Copy`
local into that `move`, a `Copy` load of a selection, or a read under an
observer (`vir-design.md`). Borrow, dereference and freeze execute over
borrow values under the prophetic model: dereference observes the borrow's
current value, mutable update rewrites the live borrow or reconciles a
consumed temporary, freeze consumes the mutable loan into a shared
snapshot. Field and variant operations execute against resolved nominal
declarations, preserve the active variant during updates, and report a
source-located error when the runtime shape does not match.

Specification numbers are unbounded integers; range, event-store,
type-domain, resource-domain and state-domain types are typed logical
nodes. Specification calls and built-ins use the closed `SpecOperation`
family with typed fields (function targets as `QualifiedRef`s, result
indexes, state anchors, memory ranges, widths), translated in both
directions without routing logical constructs through profile strings.

## Structured big-step semantics

The authoritative semantics is an inductive relation over a structured body:

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

The relation has these properties:

- a block propagates non-local control without evaluating its suffix;
- `loop` consumes a matching depth-zero break/continue and decrements outer
  nesting depths when propagating them;
- a call evaluates arguments in VIR-defined order, enters a fresh frame, and
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
came from Verex, Move, MIR, or generated source.

**Open rules.** The rules are stated over a callee oracle:
`EvalExprWith unit callee` (with `EvalValuesWith`, `EvalStatementsWith`,
`EvalArmsWith`) answers every direct and closure call by
`callee : CalleeRelation`, and the closed semantics `EvalFunction unit` is
the nested inductive tying the knot — its one rule evaluates the body under
`EvalFunction unit` itself.  `EvalExpr unit` and the other closed relations
are the instances at that oracle, so every consumer keeps its spelling
(`EvalFunction_iff` unfolds the closed semantics one step over itself). The
closed semantics is the least fixed point of the rules over the oracle,
which is what a recursive function's native denotation is proved against
([`denotation.md`](denotation.md), "Recursion").

## Interpreter

The interpreter mirrors the big-step rules but is explicitly fuelled over
loops, recursion, and calls:

```lean
interpret : ExecutableUnit -> Fuel -> FunctionRef ->
  RuntimeState -> Array RuntimeValue ->
  Except LocatedInterpError (RuntimeState × LocatedOutcome)
```

`LocatedInterpError` carries an `InterpError` (unsupported external
implementation, stuck or malformed input, `outOfFuel`), a primary `LocId`
and related call locations; `LocatedOutcome` carries the `Outcome` and its
`ExecutionOrigin`, so a language `throw_` is a successful, source-positioned
result. The interpreter executes structured VIR directly; no lowering
defines execution, a lowering is tested against the interpreter. The
foundational theorem is:

```text
interpret executable fuel f state args = ok (state', outcome)
  -> f.semantics.Relates state args state' outcome
```

`outOfFuel` makes no semantic claim and is never converted to abort or panic.

## Contracts and verification

Verification targets `FunctionMeaning`, the relational meaning read off the
big-step judgment, not the interpreter. Contracts live in VIR as
`FunctionContract`, `Condition`, `Frame`, and expression roots, interpreted
directly: `requires` constrains the initial environment, `ensures` observes
results, final referents and final state, exceptional clauses match
`ThrowKind` and the throw arguments, `old` anchors the initial state, frame
clauses constrain unmentioned state, and loop invariants attach to their
structured loop point. Quantifiers and uninterpreted specification functions
have relational denotations without being executable, so preparation for
verification accepts more than preparation for execution, and diagnoses any
construct without a logical denotation. The runtime and proof
representations of references are one prophetic ownership model
([`prophetic-references.md`](prophetic-references.md)); how a contract is
proved is [`denotation.md`](denotation.md).

For each executable declaration `f`, semantic elaboration exposes a stable
handle into the validated unit, `f.semantics` (its `FunctionMeaning`),
`f.spec` (the denotation of its contract), and `f.verified`, the theorem
`verify f` produces by resolving the handle and invoking the verifier; it
never retrieves source syntax. Calls use proved or trusted summaries
attached to VIR function identities, so callers and callees may come from
different frontends with compatible profiles.

## Lean declaration elaboration

Each validated unit is quoted once; generated declarations refer to the
unit constant and stable declaration IDs rather than expanding expression
nodes into Lean syntax, so an arena is not repeated for execution,
verification and compilation. Semantic declarations exist for every
supported function; typed façades only where a function's profile and types
admit them. Generic bodies stay generic over type, const, lifetime and
evidence; elaboration does not monomorphize them. Common semantic
combinators are ordinary definitions and lemmas, and source and provenance
tables stay out of definitional equality and proof terms.

Every generated declaration and obligation records its originating
`NamespaceId`, declaration ID and `LocId` in an environment extension, and
errors raised on generated declarations are translated back through it.
Generated subterms inherit the most specific VIR node location, with related
ranges for call site, callee, macro origin or imported MIR location; a
target node combining several VIR nodes keeps one primary location and the
rest as related ranges, and a VIR node expanding to several targets gives
each its location. Validation, the interpreter (a `throw_` at the throwing
expression, a stuck operation at its expression, callers as related
ranges), generated goals, malformed specifications and lowerings all report
at the VIR location; generated text is never the authoritative location.

## Tests

Semantic unit tests build raw VIR fixtures, validate them and run the
interpreter; frontend integration tests elaborate source to VIR and run the
validated artifact; differential tests compare VIR execution with the
linked MonoVM ([`monovm-link-design.md`](monovm-link-design.md)). Each case
records return values, throw kind and arguments, visible final state, and
the contract result, and failure cases assert their source ranges. Fuel is
a harness parameter, never expected language behavior. Successful
interpreter evaluations use the soundness theorem to produce semantic
facts; negative tests assert diagnostic codes and ranges. Cross-package
tests live in `verex-e2e-tests`, not in `verex-ir` or a profile package.

## Correctness obligations

The following claims are explicit:

- interpreter results are admitted by VIR big-step semantics (`run_sound`);
- finite deterministic VIR executions are reproduced with sufficient fuel
  (`evalFunction_complete`, `run_complete` over fuel monotonicity; big-step
  determinism of `EvalFunction` and `FunctionMeaning` follows because the
  interpreter is a function);
- semantic preparation plus well-typed inputs implies preservation and no
  stuck execution;
- interpreter outcomes, errors, and generated proof obligations preserve
  their originating VIR locations;
- calculated WP implies semantic WP;
- `f.verified` proves `Contract.Satisfies f.semantics f.spec`;
- the interpreter and the verifier run one prophetic ownership model, related
  by the agreement theorem of
  [`prophetic-references.md`](prophetic-references.md);
- Move abort finalization rolls back exactly the required state;
- each executable lowering preserves VIR outcomes;
- each frontend's alignment evidence justifies relating its source artifact
  to VIR semantics; and
- canonical source re-elaboration preserves normalized VIR semantics.

The first six are owned by `verex-ir`.

### Preservation and no-stuck decomposition

The preservation metatheory is staged as follows; `ValueHasType` is the
structural layer it builds on.

1. **Store typing.** A `StateTyping` assigns an optional `TypeId` to every
   heap slot; global slots carry their resource `TypeId` already. A typed
   value judgment strengthens `ValueHasType` at references: the target slot's
   assigned root type, projected through the reference's projections, is the
   reference type's referent. Nominal field and closure capture typing need
   the owning `ExecutableUnit`, not just `Tables`.
2. **Frame and state judgments.** `TypedFrame` relates a declaration's local
   table to a runtime frame: initialized locals are typed at their declared
   types, uninitialized locals are `none`. `TypedState` requires every live
   heap and global slot to be typed at its assigned type.
3. **Leaf lemmas.** Validation's literal/type checker
   (`firstSliceConstMatchesType`) and the reifier (`constValue?`) agree with
   `ValueHasType`; `initialFrame?` produces a typed frame from typed
   arguments. Both functions are total so these lemmas exist: the nested
   `ConstValue` recursion is structural, not `partial`.
4. **Operation lemmas.** Every semantic operation evaluator preserves typing
   and — for no-stuck — is defined on the operand shapes semantic
   preparation admits. Each `VIR-SEMANTIC-TYPE` rule in validation names the
   shapes its operation lemma may assume.
5. **Evaluator induction.** Preservation threads a growing `StateTyping`
   through the mutual evaluators; no-stuck additionally consumes the
   initialization certificate carried by `ValidatedUnit` (reads are
   initialized on every path the certificate covers) and concludes that a
   prepared, typed invocation returns a result or `outOfFuel` — never a
   stuck diagnostic.

Preservation of runtime typing for prepared units is
`Semantics/Preservation.lean`; the static type checker it rests on is
designed in [`static-typing.md`](../historical/static-typing.md).

Profile finalization laws are proved against interfaces declared there.
Lowering and frontend alignment theorems are owned by their adapters, then
composed with VIR verification. Constructing a `ValidatedUnit` alone does
not prove source or MIR alignment.

## Settled decisions

- Validated structured VIR, not retained source syntax or lower CFG IR, is the
  semantic source of truth.
- The interpreter executes structured VIR directly and is fuelled only as a
  termination device.
- The relational big-step semantics is authoritative and contains no fuel.
- Verification is stated over big-step function meaning, with a proved-sound
  calculated WP for automation.
- Runtime references and proof references are one prophetic ownership model.
- VIR semantic declarations are independent of frontend origin.
- Source locations propagate through execution, verification, and lowering;
  they are diagnostic metadata and never alter semantic outcomes.
- Generic VIR bodies remain generic over evidence rather than being
  monomorphized for elaboration.
- Generated Lean declarations are small indexed façades over one registered
  unit, not repeated expansions of the VIR body.
- Unsupported constructs fail during validation or semantic preparation with
  a location and stable diagnostic code; there is no fallback to source
  elaboration, which would hide coverage gaps.
