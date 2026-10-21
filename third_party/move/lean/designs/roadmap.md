# Roadmap and test ledger

This is the one place for project status: what is still open across the
designs, in priority order, and the ledger of the
acceptance fixtures that measures it ([Tests](#tests)). Each linked design
stays authoritative for its own scope and rationale; when a status
changes, update it here, not in the design. Done work is named in one line
per track so the open items read in context. Superseded design analyses
live in [`historical/`](historical/); session history stays in version control.

The design documents inside `v0/` (`move/`, `move-model/`, `transpiler/`)
describe the deprecated stack. Nothing is scheduled from them; they are
mined for solutions, and all their verification fixtures are ported to the
ledger below.

## 1. Verification of validated VIR

Designs: [`denotation.md`](denotation.md),
[`prophetic-references.md`](prophetic-references.md), and
[`proof-strategy.md`](proof-strategy.md).

Implemented: one denotation of validated VIR into `Spec`, compiled per target
and checked by the kernel. It covers control flow, invariant-based loops,
prophetic references, storage and resource invariants, vectors, generics,
recursion and cycles of calls. Calls are inlined unless native, opaque or on a
cycle. Callers may assume a callee's contract without its theorem; a callee
still being verified is refused, and a failed callee remains reported.

[`lemmas.md`](lemmas.md) describes specification lemmas, well-founded recursion,
and in-body `assert`, `assume`, `apply` and `split` steps. Assumptions used by a
lemma remain hypotheses of its theorem. Generic module axioms and invariants
are instantiated at the types a verification uses. Partial specification
functions retain the domains induced by their declared types; see
[`prover-test-problems.md`](prover-test-problems.md), G14–G15, for numeric
semantics and unsupported conversions.

Open work:

- **Proof strategy.** P1 (abstract maps) and P2 (source-named goals and laws)
  are implemented. The typed `SpecMap K V` laws and selective lifting support
  authored map proofs without local encoding helpers. P3 remains open: a
  complete automatic leaf attempt has a 20M heartbeat cap, and authored proofs
  bypass automatic search unless they request it. Opt-in context grouping and
  `verex_obligations preparing` share work before proof clauses split;
  `VerexSL/Tests/SharedPreparation.lean` covers clause names and soundness.
  Preparation is still expensive. P4's goal is all 38 supported BigOrderedMap
  targets below roughly 1.9G total native heartbeats, including authored proofs.
  The measured 3.582G result does not meet it; see the map measurements in
  [`perf-notes.md`](perf-notes.md). The implementation stages in
  [`shared-obligations.md`](shared-obligations.md) are planned: measure repeated
  work, share bounded automatic preparation, delay splitting, and add formula,
  proof or theory-state sharing where measurements justify it.
- **Agreement and typing assumptions.** Discharge the assumptions listed in
  [`denotation.md`](denotation.md), including the compilation agreement and
  the connection between native storage and verification values. The denotation
  uses typed global memory; deriving the remaining global-preservation
  hypotheses is tracked in [`static-memory.md`](static-memory.md) and
  [`static-typing.md`](static-typing.md).
- **Quantifier automation.** Range instances, writes, search results and
  constructive witnesses have bounded deciders. Existing call observations
  precede reconstruction of behavioral facts. Conditional self-referential
  equations remain facts but cannot drive recursive simplification.
  Instantiating every position read in a context is still costly; target
  relevant patterns and share preparation without enlarging search budgets.
- **Cost.** Incremental normalization and cached normal forms avoid repeating
  work across proof steps. Kernel certificate checking and code generation
  also matter; heartbeat totals alone do not measure all elapsed work.
  [`perf-notes.md`](perf-notes.md) records measurement methods, comparisons and
  rejected experiments. Keep automatic search cheap and use authored arguments
  for obligations it cannot decide within the existing budgets.
- **Native models and maps.** Verification reads intrinsic maps as abstract
  values; table roles execute over native handle storage at runtime. Their
  agreement and representation assumptions are explicit. `SmartTable` is
  deprecated and excluded from the gates. A0–A3 of
  [`intrinsic-maps.md`](intrinsic-maps.md) are implemented; A4 and the remaining
  construction, representation-reading and identity-update obligations are
  open. BigOrderedMap's unsupported iterator models and mutable-reference
  behavioral predicates remain separate from the cost of its supported proofs.
- **Unsupported source and contract forms.** The registry in
  [`prover-test-problems.md`](prover-test-problems.md) identifies the remaining
  denotation, native-model, generic-aliasing and closure gaps. Preserve explicit
  unsupported diagnostics and intended negative specifications while adding
  support.

## 2. Verification across modules

Design: [`relocatable-namespaces.md`](relocatable-namespaces.md).

Done: R1 (relocation: namespaces carry their own tables and validation
certificates, and a unit links them without re-validating) and R2
(a `verex module` links the registered modules it uses; verification
inlines across modules; a clause is reported in the file that authored
it).

Open:

- **R3: proof reuse.** An opaque callee of another module is verified again
  in every importer, and the unit certificates of every linked module are
  checked again (7.5 s for a client of `ordered_map`). Reusing its theorem
  needs the semantics invariant under consistent renumbering, or semantics
  over namespace-local identities; either makes a package's verification
  cost linear.
- **R4: shipped namespaces.** The exchange format carries relocatable
  namespaces, so a frontend can ship a precompiled package.

## 3. The Move standard library as the verification corpus

Designs: [`source-verification.md`](source-verification.md) and
[`proof-file-editing.md`](proof-file-editing.md).

`move prove --lean` (or `aptos move prove --lean`) exports a package's typed
AST and runs `verex-move verify --export`. The e2e `verify` suite verifies the
framework's `move-stdlib` in place; the copy under `MoveToVerexSL/MoveStdlib`
serves the source round-trip baselines. The suite's `SourceVerify/MoveStdlib.exp`
records expected diagnostics when present; its absence requires empty output.

A function that automation does not establish within its budget gets an authored
proof in `foo.proof.lean` beside `foo.move`. The file's header loads its module
context for both the driver and the editor. The corpus includes direct proofs
for `features.apply_diff`, `acl.remove`, `simple_map.add_all`, pool operations,
and the ordered-map use cases. Intrinsic vector operations use their semantic
models and contracts instead of inlining their implementation loops.

Open: reducing proof and context-preparation costs, reusing imported proofs,
and locating residual obligations that have no clause marker at the function
rather than the module header. Unsupported constructs and remaining proof gaps
are tracked in [`prover-test-problems.md`](prover-test-problems.md).

## 4. Shared checking

Design: [`vir-design.md`](vir-design.md), Phases 1–2.

Done: a single-run `validate` with authoritative name resolution,
unification typing of executable and specification bodies, and the
initialization and borrow analyses run once with certificates on
`ValidatedUnit`.

Open:

- The checked trait-selection evidence table (also blocking
  [`rust-mir-design.md`](rust-mir-design.md)).
- The certification pass that turns `prepare*` into pure mode filters.
- Borrow checking: general non-lexical loan death, dependency and unknown
  aggregate alias propagation, nominal variance, use-site region solving.
  A holder overwritten whole (an assignment or a binding of the local) no
  longer keeps its loans live, and an assignment ends the loans neither its
  place nor what follows uses once its value is produced, before the write;
  a unit with a rejected function is not verified.
- Strict external JSON compatibility of RawUnit and the full staged API.

## 5. Runtime semantics

Design: [`elaboration-design.md`](elaboration-design.md) (runtime model,
big-step semantics, interpreter).

Done: preservation of runtime typing for prepared units
(`Semantics/Preservation.lean`, [`static-typing.md`](static-typing.md)), and
loan independence: runs from starts differing in their loan bookkeeping
agree up to the loans they mint (`Semantics/LoanIndependence.lean`).

Done: a mutable reference a call returns that no local holds
keeps its current value until it dies: validation binds it to a local of
its own (`holdMutableReferences`, [`prophetic-references.md`](prophetic-references.md),
"Unheld results"). Before, the lender kept the hole at run time.

Done: a mutable borrow of a place that dies unheld reads the
place: checked as a mutable borrow, it is then read as a shared one
([`prophetic-references.md`](prophetic-references.md), "Unheld results").

Done: loan independence for lent starts and injective
renamings (`StateRenamed`, `StateRenamed.ofLent`, `evalFunction_agree`),
with comparisons reading references as their referents
(`RuntimeValue.observe`), so runs lending references agree up to loan
identities ([`static-typing.md`](static-typing.md), "Loan independence").

Open: the rest of the metatheory of the big-step semantics — determinism,
completeness of the interpreter up to fuel, and no-stuck for prepared
invocations.

## 6. MonoVM differential harness

Design: [`monovm-link-design.md`](monovm-link-design.md).

Done: M0–M3 and M4a (the linked adapter, marshalling, the three-way
differential driver, composite values); M4b in part.

Open: resources and events, and mutable-reference out-parameters (M5).

## 7. Rust frontend

Design: [`rust-mir-design.md`](rust-mir-design.md).

Open: the M0 gate (blocked upstream, below), then M1 mapper completion and
Charon differential tests, M1.5 prebuilt exporter distribution, M2
contracts and `verify` for Rust through `denote`, M3 regions over the
prophetic model, M4 drops and panics, M5 the unsafe profile
([`unsafe-pointers.md`](unsafe-pointers.md), a proposal scheduled here),
M6 library models, M7 alignment proofs.

## 8. Surface and backends

Designs: [`verex-sl.md`](verex-sl.md) and
[`vir-design.md`](vir-design.md) Phases 4, 6, 7.

Done: validated VIR lowers to XIR, and compiler-v2 compiles `.lean`
sources through it.

Open: the VerexSL forms still stated as target language (`core.*`,
`spec.*`, extension and dependency forms); retiring the NSIR bridge;
corpus-wide semantic alpha-equivalence and source maps; a
compiler-correctness theorem connecting source verification to the
emitted bytecode.

## 9. Higher-order functions

Design: [`higher-order-functions.md`](higher-order-functions.md).

Done: H1, closures at run time (masks, instantiations fixed at
construction, capture validation, `invoke` in the borrow analysis, name
ordering); H2, the Move path (closure construction in the exchange, the
Move persistence rule, VerexSL mask and generic-target spellings, XIR
lowering, MonoVM differential and compiler-v2 transactional fixtures); H3,
the denotation (function values as closures, invocations of known closures
verified through their targets' contracts, generic targets included).

Done: H4a, the behavioral predicates in contracts; of H4c, the
dispatch of a literal closure's predicates to its target's precondition and
theorem. Done: of H4d, `&mut` arguments of behavioral predicates
(`AbortsOfMut`, `EnsuresOfMut`, `ResultOfMut`), read of literal closures by
their targets' contracts and bodies; open there: a state label witnessed by
`&mut` predicates of an inlined target, invoking an unseen function value
with `&mut` parameters, and labels defined by such invocations
(`prover-test-problems.md`, V20). Done: results that are
mutable references read as their referents' values at return
(`mutableResults`, `readResults`), `result_of` of lent invocations unique,
and a call lending references or returning one named as a run of the
function value naming its target (`wp_named_mut`, `wp_named_mut_typed`).

Open: the rest of H4 — typing of invocation outcomes (H4b, planned in
[`static-typing.md`](static-typing.md), Phases 1–4 done: the checker at
preparation, defensive semantics, the operation lemmas, and preservation;
Phase 5 in progress: the invocation rule's core and loan independence done,
next the typing hypotheses, the solver rule, and the callers' discharge),
the rule for
invoking an unknown closure and the closed-world theorem (H4c), state
labels, frames, and `&mut` arguments (H4d) — then H5, the re-entrancy lock.
A function type gets a typed carrier, as `u64` has one ("Closures in memory:
a typed carrier", milestones C1–C4): no
function value needs reasoning about types, and memory with a
function-typed field is typed for free. Done: C1 (families,
memory, and executable units at a unit), C2a (compiled code at its unit),
C2b1 (closures of targets without type parameters typed in native types,
their nodes carrying their rows), C2b2 (closures of generic targets typed at
their frames, the target frame's coherence taken at creation; coherence over
the types a frame requires and its type parameters, which real frames
satisfy), C2c (generic public theorems at every instantiation coherent with
the frame their type arguments induce), C3a (`NTy.TypedAs` for every native
type, decided by evaluation; memory with enum resources and instances of
generic declarations typed), C3b0 (native function types keep which
parameters are shared references), C3b1 (a closure's native typing checks
that its instantiation is faithful), C3a2 (`TypedAs` functional, unique,
and under substitution), C3b2 (values a closed native type admits typed
semantically where the unit's readings agree, closures included), C3c0
(native nominal types record their type arguments, so a native type reads as
one semantic type in every unit, phantom parameters included), C3c (memory
with function-typed fields typed; a function value the proof does not see
typed by its carrier, a generic function assuming only that its frame
resolves an invocation's rows to scalars); next C4, the Prover tests on
stored function values. Open, not scheduled: closures whose targets return
references, typed as function types marking their shared results.

State labels (`..S |~ …`, `exists S in *`) have their own design,
[`state-labels.md`](state-labels.md): labels as `Memory` variables, defined by
state-change predicates or quantified over all memories, milestones S1–S4
(round trip, contracts, the solver's witnesses, the rest of the
`state_labels` directory). Done: S1, labels named in the
exchange, bound at state-domain binders in VIR, spelled in VerexSL
(`S |~`, `..S |~`, `S.. |~`, `S..T |~`, `∃ (S : StateDomain)`, `publish`,
`remove`, `update`), validated; every `state_labels` test re-elaborates.
S2a: quantified labels over memory bound in contracts as `Memory`
variables, ranges selecting the states predicates and specification
functions read; S3a, a label also binds the values of the `&mut`
parameters at its state; S3b, the program points (the values and states
handed to continuations at call boundaries) as hypotheses of the leaf,
tried jointly as a label's witnesses, and a copy witnessed from the bounds
its existential states where no call separates the states
(`Check/Specifications/StateLabels.lean`; the labeled `state_labels` tests
over `&mut` parameters verify as the Prover does). S2b implemented:
state-change and invocation-defined labels are memory expressions in
contracts, including when an opaque callee's contract replaces a call.
Presence and success conditions remain proof obligations (`DefinedStateLabels`,
`DefinedStateLabelErrors`, `InvocationStateLabels`, `InvocationStateLabelErrors`).
State-label decoding normalizes known memory and decoder results before
integer range checks. Constructive and context witnesses precede program-point
search. `StateLabelDecoding`, `StateLabelWitnesses`,
`StateLabelCallObservations`, `StateLabelBooleanObservations` and
`StateLabelMemoryEquality` cover these routes, including opaque callers and
false updates. Open: reads of removed resource contents
(`unmodified_memory_at_label::swap`) and the remaining S4 cases in the design.

## Tests

### Gates

Run focused targets and `scripts/smoke.sh` between edit batches. At an
implementation checkpoint, run the full gates below. Record each run's results
in the PR or CI artifacts rather than maintaining dated snapshots here.

| Gate | Required coverage |
|---|---|
| `verex-ir`: `lake build && lake test` | IR and verifier fixtures, including `DenotePerformance` and `CompositionPerformance` |
| `verex-move`, `verex-rust`: `lake build && lake test` | Profile, frontend and backend fixtures |
| `verex-e2e-tests`: `lake build && lake test` | Source round trips, source verification, Check fixtures, MonoVM and differential execution |
| Move Prover: `MVP_TEST_FEATURE=lean cargo test -p move-prover --test testsuite` | All discovered fixtures, with unchanged baselines unless an update is intended and reviewed |

A passing registry run means its diagnostics match the baselines; it does not
mean every specification is proved. Intended negatives and unsupported cases
are documented in [`prover-test-problems.md`](prover-test-problems.md).

### Check ledger

Each name is a `.lean` file under
[`verex-e2e-tests/VerexE2ETests/Check/<Folder>/`](../verex-e2e-tests/VerexE2ETests/Check/).
Each file must match the whole output in its adjacent `.exp` at the
driver's caps (180k verification heartbeats per target). Notes name what
a file covers beyond its title: intended negatives, authored proofs, and
files without `verify` targets.

| Folder | Files | Notes |
|---|---|---|
| Scalars | `Addresses`, `Arithmetic`, `BitVectors`, `Division`, `Integers`, `Literals`, `Order`, `Signed`, `SpecLogicalArithmetic` | `BitVectors`: `pragma bv`; a one-bit membership test verifies without the pragma, while a wrong postcondition and a fact about toggled bits without the pragma fail (intended). `Division`: the division algorithm, the remainder's range, and cancellation at a divisor that is not a literal; a strict bound fails (intended). `Order`: an authored proof of transitivity. |
| Structs | `Abilities`, `Invariants`, `PositionalStructs`, `Tuples` | `Abilities` has no `verify` targets. |
| Enums | `EnumPatterns`, `EnumPayloads`, `EnumRefContracts`, `EnumRefs`, `EnumResources`, `Enums`, `MatchPatterns`, `VariantFields` | `EnumRefs` is execution only. `MatchPatterns` and `VariantFields`: a value no pattern or field fits aborts with Move's incomplete-match code, in the denotation and at run time. |
| Vectors | `CertifiedReads`, `ElementQuantifiers`, `VectorBounds`, `VectorOperations`, `Vectors`, `GenericSwapRemove` | `CertifiedReads`: signed/unsigned bounds through total integer reads, zero for missing entries, and rejection of a false signed nonnegativity claim at 25k. `ElementQuantifiers`: element-wise preconditions establish indexed-read properties for value/reference vectors, signed integers, Booleans and generic equality predicates; stronger bounds and a missing-entry claim remain rejected at 25k. A target whose guards stay undecided exhausts heartbeats rather than failing fast. |
| Control | `Aborts`, `ControlForms`, `LiteralLoops`, `LoopControlErrors`, `LoopInvariantErrors`, `LoopInvariants`, `Loops`, `LoopVerification` | `Aborts`: a false contract (intended). `LiteralLoops`: loops over literal vectors with recursive specification functions in their invariants, and a nested loop with a nonlinear invariant; a wrong value fails (intended). `LoopInvariantErrors`: an invariant not established at entry and at an iteration, and one of a loop's two annotations (a `where` region and a following `spec`), both of which form its specification. |
| References | `BorrowCertificates`, `BorrowChecker`, `BorrowErrors`, `BorrowGlobalErrors`, `CorePrimitives`, `Freeze`, `Loans`, `MutableTemporaries`, `OperandLoans`, `PlaceReadErrors`, `PlaceReads`, `Prophecies`, `ReferenceComparisons`, `ReferencePatterns`, `References`, `ReturnedMutRefErrors`, `ReturnedMutRefs` | `PlaceReads`: mutable place borrows that die unheld (dereferenced, read through, compared, discarded) read their places, run and verified; one intended failure. `PlaceReadErrors`: such a borrow in conflict with a live shared loan is rejected. `ReferenceComparisons`: `==` and `!=` of mutable references compare referents at run time. `BorrowCertificates` asserts certificates, no `verify`. `MutableTemporaries`: `&mut` results discarded, matched by a wildcard, read or borrowed through, run and verified; one intended failure. `BorrowChecker`: twelve rejections at preparation, the accepted programs verified. |
| Storage | `CalleeFrames`, `CrossInv`, `GenericSpecReads`, `GenericSpecResourceArguments`, `GlobalBorrows`, `GlobalInv`, `LooseFrame`, `Normalized`, `Read`, `ResourceComposition`, `Storage`, `StorageSpecErrors`, `TableAllocation`, `TableGlobalWrites`, `TableReads`, `TableStoredInvariants`, `WrappedTables` | `TableAllocation`: tables allocated at fresh handles and filled, emptied and read through their roles, a duplicate-key abort, vector values, and a false value rejection; the callers' frames leave the allocation history open. `TableGlobalWrites`: entries added to and updated in a stored table owe their data invariants, as a plain stored field does, with one false write rejected for each, and a cached length after additions to a fresh table. `TableStoredInvariants`: eight stored-entry proofs at 25k, including phantom arguments, computed keys and both handle layouts, two false-claim rejections, and a nested table. `TableReads`: membership, scalar and generic lookup, opaque caller old-state contract, aggregate payloads, a variant's field of a read enum value, false-lookup and false-bound rejections, and a nested table; the read roles run on table values in the interpreter. `WrappedTables`: a `TableWithLength` read through the table it holds (its roles' contracts, a functional update, a widened value, a false size rejection), its role bodies run by the interpreter. `LooseFrame` carries an authored proof. `GenericSpecReads`: a generic specification function reads the resources of its call's type arguments; a claim about another type fails (intended); a generic caller calls a generic function twice at its own type parameter. |
| Calls | `AbstractClauses`, `BcsSerialization`, `Callees`, `Calls`, `Composition`, `FailedCallees`, `InlinedCallees`, `Modules`, `OpaqueGeneric`, `PreludeNatives`, `Summaries` | `FailedCallees`: a caller verified against a callee whose own specification fails (both reported as intended). Intended negatives: a caller does not see a `[concrete]` result (`AbstractClauses`), an opaque callee's body (`InlinedCallees`), a postcondition a callee of another module refutes (`Modules`), one serialized length for values of a type other than addresses (`BcsSerialization`). |
| Generics | `GenericCycles`, `GenericFunctions`, `GenericScalarCalls`, `Generics`, `GenericStorage` | `Generics`: a generic function's public theorem applied at a concrete instantiation, coherent by evaluation. `GenericScalarCalls`: scalar callers reuse the generic callee's theorem; a callee's integer result at an instance carries its type's range. |
| Closures | `Carriers`, `Frames`, `GenericHofs`, `GenericResources`, `InvariantBehavior`, `Known`, `MutableResults`, `StoredFrames`, `StoredFrameErrors`, `StoredWriteFrames`, `StoredWriteFrameErrors`, `WovenBehaviors` | `MutableResults`: a discarded `&mut` result of a call is the run its `ensures_of`, `result_of` (the value at return), and absence of aborts state; a wrong result fails (intended). `WovenBehaviors`: a leading and a trailing capture behave as the target at the woven arguments; exchanged arguments fail (intended). `StoredWriteFrames`: addressed and wildcard frames, generic resource instantiation, opaque results, and captured owners. `StoredWriteFrameErrors`: wrong resource, replacement, and false preservation claims are rejected.  `InvariantBehavior`: construction checks quantified behavioral invariants. `StoredFrames`: default function-field frames survive publication, removal, and invocation. `StoredFrameErrors`: packing and replacement reject memory-writing closures. `Carriers`: a function value the proof does not see is typed by its carrier, without a typing assumption, in a unit declaring a phantom type parameter and resources with function-typed fields; a wrong result fails (intended). `Known`: known closures invoked through their targets' contracts, a generic target included, at a concrete caller and at a generic caller's own parameters, beside a function type over those parameters; a wrong postcondition fails (intended). `Frames`: function-valued parameters keep memory, established from a target's theorem or body, over memory holding an enum resource and an instance of a generic declaration; a closure that changes memory fails (intended). `GenericHofs`: generic higher-order functions invoking a function value over their type parameters, callers at type arguments, and a value passed through; a wrong result fails (intended). |
| Specifications | `AssertionErrors`, `Assertions`, `CapturedFunctionValues`, `ContractErrors`, `DefinedStateLabelErrors`, `DefinedStateLabels`, `DomainQuantifiers`, `EncodedVectors`, `FieldUpdate`, `GenericEnumPayload`, `Increment`, `InsertionMap`, `OptionalMapResults`, `InvocationStateLabelErrors`, `InvocationStateLabels`, `Lemmas`, `RecursiveSpecFunctions`, `SpecFunctionDomainErrors`, `SpecFunctionDomains`, `SpecFunctionErrors`, `SpecFunctions`, `SpecTuples`, `StateLabels`, `TableSnapshots`, `VectorRanges`, `VectorSlices`, `WrongIncrement` | `SpecFunctionDomains`: a fixed-width parameter type is the function's domain; expansions and a recursive definition inside it verify, and the body computes beyond the width. `SpecFunctionDomainErrors`: claims at arguments outside the domain fail (intended). `VectorRanges`: vector index domains in universal/existential quantifiers and slices, shared generic values, empty/nonempty vectors, old versus current mutable values, and an excluded-upper-endpoint rejection at 25k. `TableSnapshots`: old/nested observations, caller-side quantified state labels, and functional snapshots with equal identity but different contents in one vector. `GenericEnumPayload`: a generic opaque callee returns a concrete enum; its index bound supports caller-side arithmetic after payload and carrier-family normalization. `CapturedFunctionValues`: leading and trailing captures compare executable closures with specification literals. `InsertionMap`: insertion-position roles preserve descending sequence order, with both key/rank clauses and implicit validity verified. `OptionalMapResults`: optional map result contracts and execution agree for enum tags/payloads and the legacy vector wrapper. `InvocationStateLabels`: forward and chained invocation labels, opaque callers of both projections, and invocation through a function-valued parameter. `InvocationStateLabelErrors`: a labeled projection cannot assume that an aborting call succeeds. `DefinedStateLabels`: publication, removal, update, chained definitions, generic resources, lexical lets across clauses, and opaque callers using labeled contracts without callee program points. `DefinedStateLabelErrors`: negation, implication antecedents, and an aborting removal do not supply assumptions (intended failures). `AssertionErrors`: an in-body assertion that does not hold, in a function with and without a specification, a loop body, an inlined callee, a generic function, and over a state anchor. `DomainQuantifiers`: an existential no index satisfies and one over an empty vector (intended); a frame over a vector write verifies; quantifiers over a vector type bind vectors of its element type. `EncodedVectors`: specification functions comparing vectors with literals and with each other, and a quantifier over a literal range, all verify. `Lemmas`: `spec lemma`s, a recursive one by its integer parameters and one by `decreases` with a `split`, applied in functions (`apply`, `∀ … apply`), and a proof's steps at entry and before a return; a false lemma, a false recursive lemma, an unmet premise, and an argument out of a parameter's range are reported (intended); a lemma not established gives no fact, so a function needing its conclusion fails (intended); a lemma over a vector whose induction needs its elements' bounds, and a function needing it fails without it (intended). `RecursiveSpecFunctions`: a lemma by induction and a `verify … by` using it; mutually recursive functions with lemmas recursing through each other; Boolean parameters of a function and a lemma; mutually recursive functions reading storage. `StateLabels`: a quantified state label — an existential split of a two-state specification function over two opaque calls, witnessed at the program point after the first, a one-state read at the label, and the split over two inlined calls through a `&mut` parameter, witnessed by the parameter's value at the point between them; the splits over one call fail (intended). |
| Modules | `Attributes`, `EmptyModule`, `IntrinsicErrors`, `LoweringErrors`, `Rust` | `Attributes` and `EmptyModule` have no `verify` targets. |
| Examples | `Account`, `Corpus`, `OrderedMap`, `Quicksort` | `OrderedMap` and `Quicksort` verify through authored lemmas over arrays; `Quicksort`'s `partition` runs at a raised cap of 400k heartbeats (277M in total). |

### Conventions

A Check file is VerexSL source with contracts, theorems, authored
proofs, and execution or diagnostic assertions. The driver discovers every
`Check/**/*.lean`, runs each in its own Lean process at the driver's caps,
and compares all diagnostics to the adjacent `.exp` (no `.exp` means empty
output). Run it from `verex-e2e-tests` with
`VEREX_E2E_SUITE=check lake test`; `UB=1` regenerates baselines, and every
regenerated diff is reviewed.

- A baseline records intended behavior. A negative case's expected
  diagnostic names the construct or clause, and its line carries a
  trailing `-- error: <reason>`; an unsupported positive proof is never
  recorded as an expected failure.
- A file is promoted only by the driver at the unchanged caps; a passing
  pilot promotes nothing. Caps are not raised to count a port as done.
- Every `verify` audits its artifacts (denotation route, approved axioms
  only); a fixture adds no audit of its own.
- A module lays out each function as its `fun`, its `spec`, the
  `spec fun`s that specification needs (at their first use), the theorems
  its proof uses, and its `verify … by`, in this order; a function with a
  `spec` verifies without a `verify` item. Larger modules separate groups
  of functions with `-- ## Title` headers.
- Assertion-style IR, Move, and Rust tests stay in their owning packages.
  The deprecated packages are reference material and are not run.
- Source verification is not a compiler-correctness theorem for emitted
  bytecode.

## Out-of-band items

- Abstract maps (`intrinsic-maps.md`, A2–A4): functions of an owner's module
  reading its representation through their specifications, or rejected;
  the refinement assumptions `RunsAgree` and `OwnersRepresentable` in the
  ledger and discharged automatically for units without owners;
  `big_ordered_map`'s use-case tests and their cost. Solver: an equation
  between a nominal literal and a model operation projected onto its
  fields (identity after a role's update; `perf-notes.md`, "Abstract maps:
  positions read as ranks").

- **Upstream rustc ask, filed.** The two Rustc Public API gaps blocking
  the Rust M0 gate (`FnDef` exposes no predicates; no const-fn binder type
  query) are
  [rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892).
  Until it lands the generic trait RawUnit fixture cannot satisfy the
  gate; the answer stays an upstream query, not a private `rustc_middle`
  read.
- **Security boundary.** Lean-source package discovery executes arbitrary
  host code at compile time; see the TODO in
  [`vir-design.md`](vir-design.md#todo-secure-lean-source-discovery-and-elaboration).
  It must be fixed before Lean-source discovery is a production compiler
  feature.

## Deliberately not scheduled

Phase 0 of [`vir-design.md`](vir-design.md) is a multi-party freeze with
one owner on both sides: the "no construct unclassified" gate is mechanized
by [`Capability.lean`](../verex-ir/VerexIR/Validation/Capability.lean),
and publishing RawUnit JSON v1 becomes work only when a consumer outside
this repository depends on it.

## Historical designs

[`historical/`](historical/) holds executed or superseded designs and
archives. They keep their rationale and measurements and are not updated:
[`verification-v2.md`](historical/verification-v2.md),
[`certifying-execution.md`](historical/certifying-execution.md),
[`generic-route.md`](historical/generic-route.md), and
[`verification-perf-audit.md`](historical/verification-perf-audit.md),
replaced by [`denotation.md`](denotation.md);
[`test-organization.md`](historical/test-organization.md) and
[`test-organization-history.md`](historical/test-organization-history.md),
the check ledger's earlier per-file notes, chronology, and v0 mappings,
now kept [here](#tests).
