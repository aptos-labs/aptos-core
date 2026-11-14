# Nominal invariant sorts: implementation and migration gates

> **Closed on 2026-11-13.** The migration was stopped. This record describes
> scratch copies that are not in the repository and is not updated; the last
> state and what was preserved are in the stop report under
> `/home/wrw/shared/handoff/verex-nominal-perf-2026-11-11/stop-2026-11-13/`.
> The forward plan is [`../plan/plain-values.md`](../plan/plain-values.md).

Plan: [`nominal-invariant-sorts.md`](../plan/nominal-invariant-sorts.md).

## Implementation handoff

Implementation is frozen for transfer to Fable. The complete entry point is
`/home/wrw/shared/handoff/verex-nominal-perf-2026-11-11/COMPLETE-HANDOFF.md`.
The authoritative combined source is `source-final-gate.tar.gz` in that bundle
and `/tmp/nominal-full-integration`; the main worktree remains an earlier partial
migration. Do not assume the main source matches the final compiled snapshot.

The final selected gate completed 268 jobs: 13 of 20 targets passed, seven failed.
Full soundness counterexamples and generic enum joins now pass. The runtime
forget-bridge control, opaque proof-use audit, vector source cases, labelled
update/nested-field budget checks and performance gate remain open. Plain-path
and discriminator producer drafts are separately archived and explicitly not
fully validated. No implementation agent remains active, and no final full
IR/downstream/MVP gate or native benchmark is claimed.

The sections below preserve earlier evidence; the complete handoff supersedes
statements about active work and the latest validation generation.

## Baseline

The migration starts from `78dcaf9df45`, with the folded invariant checkpoint
and closure behavior changes included. Fable's final handoff and measurements
are at `/home/wrw/shared/handoff/verex-one-spelling-2026-11-10/HANDOFF.md`.
The inherited benchmark JSON identifies that commit and records `dirty: true`;
the handoff identifies the uncommitted changes as the new plan and roadmap.
Those benchmark measurements are inherited; the IR suite was independently
reproduced below against the exact committed handoff.

Known failures must remain separate from migration regressions:

- Six benchmark failures: `ed25519`, `ordered_map`, `simple_map`, `capability`,
  `storage`, and `GenericStorage`. Preserved parent logs establish that
  `ed25519` and `GenericStorage` were already failing before the row route and
  fold. `GenericStorage` has a real rejected clause despite its log omitting
  the literal severity prefix `error:`. The analogous `storage::deposit_a`
  residual is recorded before the fold, but exact benchmark attribution of
  the remaining failures is incomplete. Evidence and required comparison runs
  are in `BASELINE-ATTRIBUTION.md` beside the measurement artifacts below.
- The fold adds an authored `OrderedMap::remove` budget failure. `add` also
  fails its configured budget. The uncapped native benchmark and the capped
  interpreted fixture have different outcomes and must not be compared as if
  they were one measurement.
- The full suites and Move Prover Lean gate were not run at that checkpoint.
  SourceVerify baseline updates are also outstanding.

A fresh detached checkout of exact commit `78dcaf9df45637cc48c447f34f96f4686bee5326`
completed the actual IR `lake test` gate (309 jobs). Only `GivenWalker` and
`StateLabelDecoding` failed. In particular, `DenotePerformance`,
`StateLabelMemoryEquality`, `StateLabelWitnesses`, `CertifiedFieldProjections`,
`GenericStorageInvariants`, `CanonicalEmission`, and the soundness and enum
suites passed. This establishes that most of the migration's initial 28 failing
files are regressions to repair, rather than inherited failures. It does not
resolve the separate downstream benchmark attribution above.

The preserved baseline checkout is `/tmp/verex-nominal-baseline-78dc`; the
complete gate log and provenance are `/tmp/nominal-baseline-78dc-ir-test.log`
and `/tmp/nominal-baseline-78dc-provenance.json`.

The user accepted the measured direction and authorized the production
migration. The inherited failures remain part of regression attribution, not
a reason to stop implementation. Do not repair them by weakening obligations,
raising proof budgets, or recording failing results as new successful baselines.

## Production migration in progress

The unified migration remains in the isolated integration package; the main
worktree still contains an earlier partial checkpoint. A full IR retry completed
449 jobs with 11 failing files, down from 28. Both enum projection suites,
Given native capabilities, frontend, soundness counterexamples, state-label
witnesses, assumption ledger and continuation audits pass together. A subsequent
15-target repair gate restored `NominalNormal`; seven selected targets remain
red, including vector/generic reads, grouped preparation and state-memory costs.
The complete performance and downstream gates have not passed.

Fable's structural analysis identifies unnecessary leaf generation, repeated
unfolding of named observations, and producer spelling inconsistencies as the
principal measured aggregate costs. An isolated path observation table now
closes `make_transfer` in the walk (one leaf to zero) and reduces `total` from
22 to four leaves, with solver heartbeats 44.661M to 9.790M against the
observer-sealed control. These are instrumented solver counts, not complete
native verifier costs. Scoped observation and cache-invalidation controls pass;
the table, collector and late vector-equation publisher also pass a combined
135-job IR gate. Two guarded enum source cases remain migration regressions. Sealing observations separately reduced costly default
unfolding while preserving the concrete and Given enum controls.

Grouped preparation had a concrete information-loss bug: duplicate field
premises simplified one another to `True` and all disappeared. Retaining locally
justified premises restores the unchanged grouped proof in isolation. Generic
storage now verifies its positive caller; a counterwrite control still rejects
an incorrect resource-preservation assertion. Exact runtime trivial-own laws
skip empty own clauses without discarding descendants or abstract predicates.
These later fixes await the next coherent combined gate.

Producer spelling remains required work. The generator must evaluate descriptor
substitution once, preserving `unmark`, and share one field-row spelling. The
executable and contract producers must emit the public projectors directly at
an exactly checked family/model boundary, rather than rely on subsequent
model-operation translation. Improving dispatch alone does not complete this.

Reproducible source, complete gate logs, structural evidence and Fable's report
are in `/home/wrw/shared/handoff/verex-nominal-perf-2026-11-11/`. No proof budgets
or performance baselines have been raised.

The first carrier foundation is checked in an isolated package build:
`Denote.ClosedSort` and the `.nominal` arms of `Denote.Types` now select a
declaration's closed datatype through `NominalFunctor.nominalClosed`. Codecs,
observation, closing, validity and generic transport use its laws. Generic
transport casts along a proved equality of closed carriers instead of opening
and rebuilding values. `NominalCertification` checks that a custom native
datatype is the actual carrier and that an impossible predicate remains empty.

The representation bridge in `NominalInvariant.Representation` is checked in
the worktree. It proves exact successful-result correspondence, preserves
aborts and undefinedness, and transports codecs without changing accepted
bytes or rejected decodes. It introduces no inhabitance assumption.

The checked-source adapter derives native predicates and row isomorphisms from
`Contract.dataOwnTerms`; its scalar `Percent` and two-clause `Band` fixtures pass
in the isolated generator package. The closed declaration is one per-source
`Sealed predicate` datatype, with a default specialization and one canonical
observer vocabulary. No second interpretation of source invariant expressions
is introduced.

The coordinated worktree build now checks the carrier foundation, schema-backed
native models, native terms, patterns, mutable paths, compiler and solver.
`NativeModelAvailability` checks that missing models make interpretation
undefined even when a pattern could fall through or a loan holder is absent.
Compiled functions remain independent of a particular carrier family; the
same function is interpreted at runtime and generic recursive-call frames.

`NominalClosing` runs the production `verex_obligations` walker on all five
certification kinds. Its audits check fresh closed continuation binders and
observation equations, including hidden arguments, and reject exposed closing
applications. Invalid and false-own predicates are rejected. The checked
proofs use only the existing standard logical axioms.

The worktree selects monomorphic scalar structs through a shared checked-source
classifier. It generates distinct native open and closed sorts, supplies the
actual source layouts as model metadata, and retains structural layouts only
for invariant translation and declarations not yet migrated. Atomic native
descriptors may reduce to their handle and arguments; this never expands the
payload layout or the closed representation.

Source-generated contracts, recursive specifications and lemma templates are
specialized to their source unit, with explicit model coverage for arbitrary
Skolem and given interpretations. Importing units regenerate source-indexed
templates. Opaque specification boundaries retain their established argument
representation.

Generated boundary equations give execution, contracts and invariant clauses
one native field spelling. The invariant opener selects the applied source
predicate before the general validity fold; unrelated validity facts remain
folded. Native open equality is interpreted through proved field equations.
Authored storage proofs normalize definitionally equal resource reads to the
same contextual rewrite key. Dependency collection traverses every native
constructor, including calls beneath native field projections.

The scalar source checks cover construction, reads, temporarily invalid mutation
repaired before closure, hand-over, copy-out, returned references, loops, typed
storage, generic calls, imported contracts and function-body lemma application.
Expression audits cover prepared read, copy, call, loop and memory obligations.
Invalid construction, exit, hand-over, copy-out and returned references are
rejected at their source boundaries without creating proof artifacts. A false
invariant has an empty closed sort and generates no unconditional inhabitant.
The existing public satisfiability assumptions remain explicit.

The finite classifier and reusable monomorphic model libraries are checked,
including acyclic dependency ranks, exact descendant validity, empty enums,
container and ghost fields, and handles-aware equality that ignores ghost
contents. The next isolated generation also checks the actual source type
reader, GlobalTyping and IndexedTyping, canonical finite quotation, and the
Contract and Solve dependency chains. Specification constructors, variant
tests, payload selections and updates consume checked finite enum models;
model selection uses exact finite source coverage and productivity. The
focused typing, quotation, model-selection and ghost-equality regressions pass.
These files remain in `/tmp/nominal-finite-integration` until the complete
production provider passes; they have not replaced the worktree's scalar
checkpoint.

The finite production provider now passes ordinary nested-struct and
three-variant enum construction, reads, matching, and mutable field replacement.
Prepared nested and enum read goals pass the representation audit. Three new
negative source cases reject invalid enclosing bounds and nested struct/enum
mutations without publishing proof artifacts. Scalar production examples still
pass. The larger existing invariant regression suite exposed repeated source
metadata reduction in contract generation; compact kernel-checked per-owner
selection equations fix that recursion-depth failure. The no-op mutation
postcondition now passes after exposing the comparison's one-layer metadata.
Pure linked lemma templates pass with an explicit checked productivity equality
in their model lookup proofs. Invariant-bearing recursive specification templates
now pass at their original budget after metadata-only preparation makes the same
compact certificates available before payload generation. The prewarm is
idempotent, checks exact unit/source identities, and emits no payload or function
family; invalid metadata is rejected synchronously without registry pollution. False-invariant emptiness, failed closing, missing unconditional
instances, and the impossible satisfiability ledger all pass through public
observation laws. The complete finite integration cone is frozen green as a
reference. It will be superseded by the unified generic source port, avoiding a
separate full rebuild of a provider immediately being retired. The combined
source still requires the Lake gate before porting. No proof budget or recursion
limit has been raised.

The generic foundation now also builds through the real `VerexIR` dependency
cone in `/tmp/nominal-full-integration`, including term semantics, GlobalTyping
and IndexedTyping. Source selection uses exact declaration arity and a static
productivity profile, retaining phantom and optional parameter behavior.
Kernel-checked source-body equations over arbitrary argument descriptors pass
for structs, enums, phantom parameters, vector fields and nested generic ADTs;
wrong layouts and arities are rejected synchronously before caching. Generic
provider, contract and solver integration remains in progress. Generic codec inverse, tightness, decoder congruence, ordering,
erasure equality, canonical views and substitution have separate checked
modules. Source admission must additionally check legal declaration arity and
type binders. Handle-aware generic equality must receive the raw argument row:
its structural handle rule can change when parameters are instantiated, even
when the native argument sorts coincide. These interface changes are staged
separately; standalone model proofs do not establish production source coverage.
The generic factory now has checked two-parameter native struct and enum
presentations, including exact models, arbitrary owner and parameter validity,
canonical views, codec laws and both runtime equality modes. Admission also
needs argument codecs to encode plain values at each argument's unmarked type;
observed markers and unconstrained parameters otherwise admit invalid test
dictionaries. These requirements follow from the existing production codecs.
A registered-source regression confirms that a phantom wrapper or an enum's
empty first variant can be a legal generic-call argument even when its own
function-type argument has no default. The former atomic test, requiring every
argument to be inhabited, would reject those calls. An isolated core migration
now checks a source-indexed productivity profile (`never`, `allArgs`, or needed
parameter indices), partial plain-value dictionaries, and profile-preserving
substitution. The profile is part of nominal identity and exact source support;
wrong profiles cannot claim a source model. Admission checks the profile at the
actual argument types before accepting a default witness. This preserves the
existing TypeArgs check and keeps closed defaults under the existing
satisfiability assumptions.

The unified generic provider in `/tmp/nominal-full-integration` now checks
validated source declarations through representation, operations, models,
source coverage, admission, bootstrap and final function-value installation.
One emitter covers arity-zero and arbitrary-arity structs and enums, phantom
parameters, nested generic ADTs, containers, ghosts and canonical intrinsic-map
owners. Wrong arity/profile keys remain unsupported. Obsolete monomorphic
emitters are retired in the scratch package with replacement coverage; the
worktree remains at its earlier scalar checkpoint pending the combined Lake gate.

The unified package passes the actual Lake core/model gates and the complete
frontend build. Required raw and derived model laws now include runtime equality
for both handle modes. A counterexample retains valid source codecs and
specification comparison while corrupting handle comparison, demonstrating why
the additional model law is necessary. Full-instance validity generation and
the walker fast path also pass their combined tests; absent own clauses never
bypass descendant validity.

Source checks pass scalar construction, reads, repaired mutation, imported
contracts, two-clause invariants, false-invariant emptiness/rejection, generic
closure captures, storage indexing, descriptor quotation, and the negative
finite-invariant cases. Nested reads and enum construction/discrimination use
the original authored tactics. Field and variant naming preserves nonconflicting
source names and source codec tags. Proof-copy lemmas now initialize checked
metadata before translating statements, preserving their original proof budget.

The actual combined Lake source gate now passes all six finite nominal targets,
including nested reads, enum construction/discrimination and repaired enum
mutation. Nested intrinsic-map copy-out also passes its original automatic
budget. The map fix prioritizes newly derived facts mentioning the target's
local data, retaining every candidate and the same checked definitional-equality
acceptance. Its isolated matching phase falls from roughly 9.9M to 8k heartbeats;
the previous cost was rejecting an unrelated final-value fact before finding
the matching initial-value fact.

The combined focused Lake gate passes all 248 jobs, including the complete
Given ADT fixture, vector mutation with all postconditions, intrinsic maps,
finite nominal mutation, storage, proof copying, negative cases and the exported
assumption ledger. The vector field-reading tactic now recognizes registered
native observations without opening their representation. Real call-result and
by-value loop program-point audits check direct closed binders and exact runtime
validity before preparation; source observation audits additionally reject
persistent codecs.

The first full IR suite completed with failures in 28 test files. These include
real regressions, stale low-level test setup/assertions, and budget regressions;
they are not accepted baselines. Diagnosed common causes include an unchecked
nominal-family respelling, incomplete declaration quotation, retained structural
Given predicate extraction, enum projection/discriminator facts, and bounded
mapped-vector lookup normalization. The first three source repairs and their
focused regressions pass in isolation; their combined full-suite retry remains
outstanding.

The existing cost gate reports native ADT and storage regressions while primitive
and modular-call costs remain close to baseline. Failed verification targets are
not counted as successful cost measurements. Stage diagnostics are guiding the
remaining work; no budgets or performance baselines have been raised. This is
not a completed migration or a demonstrated whole-program speedup.

Concrete function fields with behavioral invariants and embedded mutable
references remain supported through the structural path. Their full nominal
migration needs additional family and loan-view interfaces; that scope decision
is pending. Abstract model capabilities have become stronger even though no new
axioms were added. Generated concrete dictionaries prove their model laws;
Given coverage remains explicit. The exported-theorem signature audit passes in the combined focused gate.

The worktree still contains the earlier scalar checkpoint. The unified source
and retirement manifest will be ported whole-file after the owning gates pass.
The complete package/downstream suites, smoke, Move Prover gate, assumption
ledger and production benchmarks remain outstanding.

## Dependency inventory

| Area | Existing dependency | Migration |
|---|---|---|
| `Denote/Types.lean` | `NTy.carrier`, `forget`, `ofDeepHolds`, round trips, subtype equality | Retain the correspondence laws; introduce nominal interfaces before replacing representation |
| `Denote/Term.lean` | `wp_certifyingAt_intro_holds` passes a closing application into the continuation | Derive fresh-closed-binder rules from the existing certifying semantics |
| `Denote/Solve/Normal.lean` | Subtype projections/equality, certificate-free tests and reconstructed field reads | Canonical observers and deliberately oriented ADT laws |
| `Denote/Solve/Base.lean` | Destructuring and repeated certificate-clause extraction | Keep closed binders; expose invariant facts through nominal validity laws |
| `Denote/Solve.lean`, `Solve/Invoke.lean` | Certifying cuts, call result and loop binders | Consume fresh values and boundary equations without reconstructing certificates |
| `VerexSL/Contract*`, `Verify/{Contracts,Members,FunctionValues}.lean` | Carrier-dependent binders, observations, declaration predicates and instances | Generate canonical ADT operations shared by code, contracts and invariants |
| Codecs, `Resources`, `Interpretation`, `GivenOwn`, `Agreement` | Typed storage, reframing and interpretation transfer | Prove compatibility; preserve current public assumptions |

## Prototype boundary

`VerexIR.Proofs.NominalInvariant` provides the representation-independent
boundary and its generic WP equivalence, proved directly from `Spec.certified`.
`VerexIR.Tests.NominalInvariantSorts` constructs real distinct closed/open
types against this interface. Private guarded constructors still use proof fields inside
their implementation; the claim being tested is that those fields and closing
applications do not occur in continuation value expressions. These isolated fixtures do not establish whole-program performance; the
production migration status is described above.

The prototype uses the production `verex_normalize` pass with local observation
laws. An expression audit rejects exposed subtype terms, private representation
projections and closing applications in its tested continuation goals. A
nested observation law must remain outside the normalizer when its orientation
would replace a closed projector by an open-payload access. The explicit law
remains available for proofs at the boundary.

`VerexIR.Tests.NominalInvariantPipeline` extends the check through
`verex_prepare_leaf` and `verex_grind`: reads, nested generic projections, a
call-result equation, a pair representing loop state, function-indexed memory,
array selection and closing. It audits the prepared target, every local type
(including implicit arguments), and non-proof let values. These are hand-written
binder-shape tests, not generated source programs. They deliberately supply
validity using the nominal laws; automatic selection of those laws in the
production generator remains to be implemented.

## Integration repair gate

After the first full IR suite exposed 28 failing files, a coherent Lake repair
batch passed 14 of its 16 selected test targets. The checked fixes cover Given
source-owned predicates, native family transport, declaration quotation,
quantified mapped-vector reads, scalar reader canonicalization, and generated
enum observation facts. `CertifiedTableReads` passes in full, as do the unchanged
Given vector read/search suites, `Behavior`, `SharedBehaviorJoins`, and
`GivenFailureDiagnostics`. Native invariant rejection and finite/Given ADT
production tests also pass with these fixes combined.

The two remaining failures in that batch are `StateLabelMemoryEquality` (its
original heartbeat cap) and `StoredTableInvariantErrors.same_handle` (an
unproved clause). This is a focused repair gate, not a rerun of all 28 failed
files or a claim that the complete suite passes. A public-observer syntax guard
removes expensive failed closed-observation rewrites on open mutable values;
two measured continuation steps fall from about 4.2M heartbeats each to 1.9M,
but the state-label example still exceeds its original total budget. No
performance baselines or budgets were changed.

## Controlled measurements

Artifacts, sources and reproduction instructions are in
`/home/wrw/shared/handoff/verex-nominal-invariant-sorts-2026-11-11/`:
`MEASUREMENT.md`, `RESULTS.md`, `measurements.json` and `isolated-sources/`.
All 28 isolated cases passed; eight selected repeats differed by less than
0.03% in heartbeats, with identical proof object counts. Sequential batches
showed order-dependent proof sharing and are excluded from the comparison.

With identical native open payloads, nominal closed sorts use 12.7% fewer
heartbeats for a read, 16.3% fewer for a nested generic read, 6.0% fewer for
construction, 12.2% fewer for function-indexed memory, and 5.6% fewer for array
selection. The original Option-based enum elimination was 7.4% more expensive,
although its proof was smaller. Direct result elimination through `matchOn`
and `matchOn_rule` resolves that regression on the measured case: 282,249
heartbeats versus 326,371 for the exposed subtype with the same supporting
definitions (13.5% less), and 488 versus 577 proof objects. A repeat used
282,234 heartbeats and 488 objects. `matchOn_amount` proves equivalence to
the prior observation-based match. The richer case/observation interface is
retained for branches that need facts about the original closed owner.

A separate field-congruence experiment over 1, 4, 16 and 32 fields distinguishes
the structural-layout change from the certification change. Native payloads
remove the field-count growth. Replacing an exposed subtype over an already
native payload with a nominal closed sort saves about 22% at each width. The
much larger structural-to-native gain at 32 fields must not be attributed to
the subtype change alone; at one field the ordinary native case is slightly
more expensive than the structural one.

These are synchronous Lean theorem measurements through the production
preparation/decider tactics, excluding imports and interface-law generation.
They do not measure generated source verification or establish an OrderedMap
speedup. The prototype supports continuing to single-declaration generation;
the result does not establish efficient elimination for every enum shape.

## Single-declaration generation

The opt-in emitter `VerexSL.Verify.NominalInvariant.emitNativeStruct` accepts
an elaborated native field schema and a predicate over the generated open
structure. It emits named open/closed structs, guarded private representation,
canonical projectors and their laws, the boundary, construction and a fresh
binder WP theorem. `wp_make_fields` specializes that cut to one canonical
equation per field, in schema order; `observe_eq_of_fields` proves that these
equations determine the entire open observation. The original `wp_make` remains
the semantic whole-observation theorem. It does not register a VIR type or
change production carrier emission. Supported inputs are monomorphic structs with closed field
types in `Type`; the caller supplies the full validity predicate, including
descendant validity where needed. No codec or recursive/generic/enum adapter is
claimed by this stage.

`VerexSL.Tests.GeneratedNominalInvariant` exercises a generated `Percent` and
a two-field `Band`, canonical normalization, invalid closing, temporarily invalid open values,
sort separation, type/schema errors and generated-law axioms. The emitter
rejects predicates generalized over ambient section parameters and rolls back
failed generation rather than retaining a partial declaration.

The first generated construction cut required consumers to derive a field
equation from the observation equation, costing 331,845 heartbeats versus
275,748 for an exposed subtype. Moving that derivation into the generated
field-form theorem reduces the construction to 274,509 heartbeats versus
275,883 in a fresh-process comparison with the same supporting definitions:
essentially cost-neutral, with 528 versus 575 proof objects. The expanded
one-field interface takes 373,118 heartbeats to generate once per declaration;
that cost is recorded separately and must be included in whole-program tests.

The unified generic scratch now checks full source closure capture generation
for generic records/enums, nested owners, and phantom parameters containing
function types. Its boundary/direct-eliminator regressions cover arbitrary raw
descriptors, fresh two-field continuations, empty owners, and later live enum
variants. The canonical operation generator passes ten-owner shape coverage,
including empty payloads and ghost fields. These are isolated gates; the final
combined source verification and Lake port have not passed yet.

The current integration gap is source predicate normalization: the existing
guarded `OwnAtArguments` clause expects its source-layout value, whereas native
ownership initially supplies a native encoding. Shared encoding-preserving
plain views now compose structs, enums, containers, tables, and ghosts, keeping
parameter positions unchanged. A checked guard-preserving clause bridge removes
decoding once that view is supplied. The per-owner bridge and Given consumers
are being integrated. Checked operation dispatch also handles schema equations
that ordinary simplifier instance transparency cannot match.

## Remaining gates

- Complete finite and generic source enrollment, including nested ADTs, enum
  payloads, ghost fields, containers, abstract table owners and storage.
- Preserve source typing and exact operation semantics at every model boundary,
  including specification equality and legal generic instantiations.
- Extend the source expression audits to these shapes and to Given/concrete
  transfer, exhaustive matching, nested loans and reborrows.
- Remove the scalar-only provider, transitional structural ADT representation,
  and migrated compatibility/repair rules after their replacements pass.
- Run all four package suites, smoke tests and the Move Prover Lean gate;
  distinguish inherited failures from migration regressions.
- Measure complete generated-program and authored-proof costs, including
  interface generation, and compare the plan's benchmark groups. The prototype
  measurements do not establish an OrderedMap improvement.

The remaining D1 plain-binder migration stays paused; its landed fold remains
the baseline while this replacement is completed.
