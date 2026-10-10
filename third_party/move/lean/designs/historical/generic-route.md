# The generic route: verification by normalization, not by stepping

> **Historical (moved here 2026-09-08).** This document tracked the
> normalization route and, from 2026-09-05, the native route that replaced
> it; both are superseded by [`../denotation.md`](../denotation.md) (V5: one
> denotation of validated LIR and one agreement proof). Kept for the pilot
> measurement, the checkpoints, and the native cutover census; no longer
> updated.

## Native migration priority (2026-09-07, superseded by `denotation.md`)

Native-encoding migration precedes further v0 ports and performance tuning of
the retiring representation. Source verification is now **native-only by
default**, as explicitly requested by the user. `script`, `normalize`, and
`compose` are rejected before preparation or cached-proof reuse. There is no
compatibility switch. Unsupported native bodies fail visibly until migrated.
Every successful `verify` automatically audits its computation/VC dependencies;
failed generation or auditing rolls back the published proof artifacts.

The 8,201-line `RowScript.lean` generator and 1,080 lines of retiring route
selection/emission in `Contract.lean` have been removed. Shared options live in
`LeanerLang/Options.lean`. Acceptance fixtures and the composition benchmark now
request native verification; changing their route does **not** make them migrated.
The cutover census completed: **17/61 Check files pass; 44 fail**. Of the
failures, 42 expose native implementation/proof gaps and two only differ in
expected diagnostics. Six IR test targets fail; the Move and Rust suites pass.
These are affected files, not independent bug counts. Earlier passing mixed-route
audits below are historical, not evidence that the native-only suite is green.

The latest generated scalar-owner mutation step (2026-09-08) retains the native-only audit at
**19/61**: `Verification/Loops` now passes completely, as does
`Language/Integers`; the indexed targets and
`ControlForms.compound_local`, `clamp`, `then_else`, and `checked_assert`, plus
six state-free `Verification/Aborts` targets, plus ten v0 `Language/Loops`
targets (and two return-scope regressions) and two `Verification/LoopInvariants`
targets, pass inside still-incomplete
fixtures. Forty-two files fail (39 native gaps and three diagnostic-only
mismatches, now including the rejected loop-invariant contracts). The same six IR
targets fail; dedicated native-computation regression/cost gates, Move (71 jobs),
and Rust (165 jobs) pass. Direct literal writes through one fixed-width integer
reference parameter now generate native owner outputs and authored contracts
automatically. Checked replacement expressions, reborrows, owner-returning
calls, and `clear`/`drain` remain incomplete.

Extend shared native combinators and their separate execution-agreement laws,
then move acceptance targets to explicit native verification. Remaining work
includes control flow, heterogeneous/aggregate/generic locals, references and
typed storage effects, modular calls across those effects, recursion, and loop
invariants. Enforce native-artifact dependency audits before retiring old VC
routes; preserve runtime frames in execution/agreement where required. Keep
the existing acceptance caps and performance baselines. The uncommitted
returned-reference correctness changes remain in scope, but further legacy
frame-specific tuning is not the migration strategy.

Removal exit criteria (not yet met):

1. Every supported acceptance verification target must select `native` and pass
   `#leaner_require_native`, including agreement and VC dependency checks. The
   historical 60/61 mixed-route result is not a native-coverage count; the
   native-only cutover passed 17/61 files, now improved to 19/61.
2. Complete the remaining consumers with fallback already disabled. Default
   native, rejection of retired route selections, cached-proof checks, and
   automatic artifact auditing are implemented; native coverage is still incomplete.
3. `RowScript` and the normalization/composition source VC generators are
   removed. Retire their remaining proof-law modules and manual row tests after
   migrating any shared consumers. Execution agreement and contract/state
   adapters still need a dependency audit; do not delete required semantics.
4. Keep execution semantics and the exact native-to-execution agreement laws;
   runtime frames in those laws are not a fallback VC representation. Typed
   native storage/effect migration must also replace the compatible state and
   contract adapters before claiming the complete representation migration.
5. Run the existing functional, negative, acceptance, and performance gates
   without raising budgets or refreshing baselines to hide regressions.

The deprecated `move`, `move-model`, and `transpiler` packages remain read-only
reference under `CLAUDE.md`; this removal concerns the active leaner verifier's
retiring paths, not deleting those reference packages.

First migration step (worktree): shared native conditional and comparison
generation, including Boolean results and mixed Boolean/integer locals.
`Language/Arithmetic` moves five additional targets to explicit native
verification, and `Language/ControlForms` moves `branch_effect`. Fifteen new
native control-flow checks cover nested branches, local bindings, false
contracts, and selected/unselected arithmetic aborts. They cost 1.5–6.3M raw
heartbeats including all generated proof stages, under the unchanged 50M cap.
The new `#leaner_require_native` command rejects legacy typed wrappers and
audits computation/VC dependencies; the existing four generic native proofs
also use it. This is not complete migration of either source fixture or of the
compatible runtime store/failure vocabulary. IR functional/native cost checks,
Move, and Rust suites pass; the legacy Performance gate retains its seven
known regressions. No performance baseline was changed.

Second migration step (worktree): monomorphic native struct/enum constructors,
nested construction, aggregate locals/results, enum predicates, and payload-free
classification. Nine positive native aggregate checks, false-contract rejection,
and four exact native execution assertions pass. All generated stages cost
0.84–3.18M raw heartbeats per positive target. `Language/Enums` now selects native
for `make_transfer`, `classify`, and `is_transfer`, with no-fallback artifact
checks; the complete file and its interpreter assertions pass. Payload extraction
(`total`/`mixed_match` and `EnumPatterns.nested_total`) is still to migrate.
Nominal erasure and parameter-loan metadata remain confined to agreement;
arbitrary generic codecs are not assumed loan-free. The native branch WP rule
selects the outer `if`, without accidentally splitting a match inside its test.
The next payload step must compose projections inside conditions and arithmetic,
not just bind an extra local: the lowered nested matcher contains variant tests
of projected enums and addition of recursively projected integers. Reconstruct
typed pattern matching with path-specific projection certificates. v0's native
matches are the reference; its arbitrary value for an invalid *specification*
field selection is not an exact execution certificate for a runtime projection.
Validation after this step: full capped Check audit **59/61**, with only the
same unmigrated `EnumPatterns` and `Generics` timeouts. IR functional/native and
composition-cost checks pass; the legacy Performance gate still has the same
seven regressions. Move (70 jobs) and Rust (119 jobs) pass. No checkpoint commit
or source-port promotion has been made for this worktree.

Third migration step (worktree): native enum payload matching and owned struct
destructuring, including nested patterns, arithmetic on selected payloads, and
typed payload guards. Selected enum arms carry actual native constructor fields;
the separate agreement uses a checked branch-selection law and exact projection
certificates. No arbitrary payload is supplied for an unselected variant.
Modular owned calls accept native constructors/projections and consume only the
callee's native summary, not its body. Contract-boundary normalization reduces
literal payload array reads; selected-path premise normalization does not revisit
the whole WP. Range certificates are exposed as arithmetic bounds when native
payloads enter scope.

Ten positive payload/destructuring tests, two rejected contracts with artifact
rollback checks, and four exact native execution assertions now pass cleanly.
All generated stages cost 3.31–43.19M raw heartbeats per positive target under the
unchanged aggregate 50M gate; the four modular callers cost 4.06–4.81M.
The acceptance edits select native for the remaining two `Language/Enums`
targets, all six `Language/EnumPatterns` targets, and both verified
`Language/PositionalStructs` targets, each with `#leaner_require_native`.
Integration validation of this payload step passes **60/61** Check files; only
the existing `Language/Generics.choose` timeout remains. All three newly native
enum/struct fixtures pass with their interpreter assertions. IR functional,
native-cost, and composition-cost tests pass; the legacy Performance gate still
has the same seven regressions. Move (70 jobs) and Rust (121 jobs) pass.
Generic instantiated twins,
reference/storage effects, recursion, and loop migration remain open. No source
port counts or performance baselines have been changed.

Fourth migration step (worktree): aggregate forwarding and modular native calls
in local bindings. The shared native call emitter now supports continuations;
it replaces the separate integer-only local-call generator. Typed locals can
hold owned aggregate call results, feed them to another callee, or match them.
Earlier aggregate-value equations from summaries are reused at contract
boundaries, without rewriting arithmetic numerals or unfolding callee bodies.
Proof-producing premise normalization explicitly maintains its successor goal.

Eight positive owned-call checks, two rejected contracts (including a result
not promised by a weak callee summary), native-artifact and summary-reuse checks,
and three exact execution assertions pass. All generated stages cost 1.00–13.01M
raw heartbeats; two aggregate calls cost 8.25M and four cost 13.0M, within the
unchanged 2× scaling guard. Existing native local-call and payload cost checks
also pass. The owned-call integration audit passes **60/61** Check files, again
with only `Language/Generics.choose` over budget. Move (70 jobs) and Rust
(122 jobs) pass; IR reports only the same seven legacy Performance regressions.
No budgets, baselines, or source-port counts were changed.

Fifth migration step (worktree): Boolean equality/patterns and modular Boolean
call results. Equality dispatch uses the typed operands: the lowered operation
descriptor records its result type, so it cannot distinguish integer equality
from Boolean equality. Native guards reuse callee truth equivalences, and
constructor-backed specification projections normalize before Boolean clauses
are split. Guard normalization has a single direction, with a small cost-capped
regression against mutually inverse false/not-true rewrites.

Nine positive Boolean tests, one rejected false contract with artifact rollback,
summary-reuse checks, and four exact native execution assertions pass at
1.34–5.06M raw heartbeats across all generated stages. Existing native branch,
payload, owned-call, and local-call cost guards pass. `Language/ControlForms`
moves `match_effect`, `echo_flag`, and `if_let_action` to explicit native with
artifact audits; the full file passes. Move (70 jobs) and Rust (122 jobs) pass;
IR reports only the same seven legacy Performance regressions. The complete
Check audit passes **60/61**, with only `Language/Generics.choose` over budget. No caps,
baselines, source-port counts, or checkpoint status changed.

Sixth migration step (worktree): effectful fields in native struct/enum
construction, including nested constructors and mixed Boolean/integer fields.
Typed operand products compose with `Spec.bind`; a shared scalar-operation law
certifies their exact evaluation order and abort behavior. The existing pure
constructor path remains available. A named-pure-value WP rule carries the
exact aggregate into local continuations and modular calls.

Seven positive tests, two false contracts (including incorrect operand-failure
order), summary-reuse/artifact audits, and four exact execution assertions pass.
All generated stages cost 1.31–6.16M raw heartbeats; each execution assertion also
passes a separate 1M cap by normalizing computational bind laws before observing
the result. Existing native cost guards pass. `ControlForms.embedded` selects
explicit native and the full fixture passes. The complete Check audit is
**60/61**, with only `Language/Generics.choose` over budget; Move (70 jobs) and
Rust (125 jobs) pass. IR reports only the same seven legacy Performance
regressions. No caps, baselines, source-port counts, or checkpoint status changed.

Seventh migration step (worktree): effectful arithmetic comparisons, branch
conditions, local Boolean results, and nested short-circuit conditions. Shared
scalar and function-boundary branch laws preserve evaluation order, intermediate
states, and condition failures before either branch runs; they do not equate
abort states to entry. Constructors and comparisons share typed operand products.
Guard normalization handles named Boolean equations and unreachable constant
guards; false Boolean equivalences with compound clauses become negations before
refutation, while arithmetic equations retain their existing closing path.

Nine positive tests, two rejected contracts with rollback checks, and seven
exact execution assertions pass. All generated stages cost 4.69–7.35M raw
heartbeats; each execution assertion passes a separate 1M cap. The four
`ControlForms` targets `arithmetic_condition`, `explicit_arithmetic_condition`,
`short_circuit_and`, and `short_circuit_or` now require explicit native artifacts;
the full fixture passes. Move (70 jobs), Rust (127 jobs), and IR functional/native
cost checks pass; IR still reports only the same seven legacy Performance
regressions. The full Check audit passes **60/61**, with only
`Language/Generics.choose` over budget. No caps, baselines, or source-port counts changed.

Eighth migration step (worktree): eager Boolean conjunction/disjunction and
negation, including arithmetic operands, local conditions, and left-to-right
operand failures. Typed operand products evaluate both arguments even when
the first determines the Boolean result. Native guard normalization preserves
compound truth facts; Boolean-to-arithmetic normalization retains its successor
goal and closes arithmetic before exposing integer implementation details.

Nine positive native proofs, two rejected contracts with artifact rollback,
and five exact execution assertions pass. All generated stages cost 1.65–7.77M
raw heartbeats; exact assertions each pass a separate 1M cap.
`ControlForms.eager_core_and` now requires native artifacts, and its complete
fixture, including interpreter assertions, passes. Move (70 jobs), Rust
(127 jobs), and IR functional/native cost checks pass; IR retains only the same
seven legacy Performance regressions. The full Check audit passes **60/61**,
with only `Language/Generics.choose` over budget. No caps, baselines, source-port
counts, or checkpoint status changed.

Ninth migration step (worktree): native division/remainder, including zero
divisors, signed truncation toward zero, and quotient overflow before remainder.
A shared typed result computation and agreement-only result conversion preserve
ordered operand failures without duplicating match helpers in each certificate.
Range facts come directly from the native operand certificates. Arithmetic
evaluation facts moved out of the retiring WP module into `IntegerEvaluation`,
so native agreement no longer needs that legacy dependency.

Thirteen positive proofs cover scalar operations, nested operands, locals,
constructors, conditions, repeated remainders, and a modular call continuation.
Two false contracts are rejected with artifact rollback, a summary-reuse guard
passes, and seven exact execution assertions pin signs, zero-divisor payloads,
quotient overflow, and operand order. All generated stages cost 2.41–7.03M raw
heartbeats; exact assertions each pass a separate 1M cap. `Arithmetic`'s two
scalar division/remainder targets and `Signed`'s two corresponding targets now
require native artifacts; both complete fixtures and interpreter checks pass.
Move (71 jobs), Rust (132 jobs), and IR functional/native cost checks pass;
IR retains only the same seven legacy Performance regressions. The full Check
audit passes **60/61**, with only `Language/Generics.choose` over budget.
No caps, baselines, source-port counts, or checkpoint status changed.

Tenth migration step (worktree): native scalar literals share the pure-value
generator and exact literal agreement with constructors. This replaces the
Boolean-only terminal case and admits zero-parameter integer, address, string,
and byte-array constants. Contract translation also accepts byte-array literals
in its existing aggregate domain; computations return native `Array UInt8`, not
encoded values. Eight positive proofs, two false-contract rejections with
artifact rollback, and six exact execution assertions pass. All generated
stages cost 0.84–2.30M raw heartbeats, and each execution assertion passes a
separate 1M cap.

Eight `Language/Integers` targets and `Literals.fixed_address` now select native
and require native artifacts. Both complete fixtures and their interpreter
checks pass. All ten `Verification/GenericScalarCalls` targets also select
native; eight dependency guards confirm concrete callers reuse the generic
callee's `pureVerified` theorem. Existing scalar arithmetic targets now carry
the same artifact audits. IR functional/native cost checks, Move (71 jobs), and
Rust (132 jobs) pass; the seven legacy Performance regressions remain. The full
Check audit passes **60/61**, with only `Language/Generics.choose` over budget.
No budgets, baselines, source-port counts, or checkpoint
status changed.

Eleventh migration step (worktree): fixed-width native AND/OR/XOR, with signed
interpretation, nested masks, local bindings, conditions, constructors, ordered
operand failures, and modular call continuations. A shared wrapping theorem
certifies every result's range; a separate law proves exact agreement with the
bit-pattern evaluator. Native bitwise results remain folded until their checked
mathematical equations apply, so choosing contract witnesses does not discard
the range certificate. Unsigned AND has a direct mathematical rewrite; named
integer locals retain a scalar value equation across modular calls.

`NativeOperands` now owns typed operand products, their left-to-right WP
composition, state preservation, and separate agreement generation. Arithmetic,
constructors, and conditions share this implementation instead of duplicating
the product/sequence machinery. Thirteen positive bitwise proofs, two rejected
contracts with rollback checks, one modular-summary reuse guard, and seven
exact executions pass. All generated stages cost 2.23–5.65M raw heartbeats;
exact execution assertions each pass a separate 1M cap. Existing native operand,
division, and other IR native/composition cost guards pass after consolidation.
`Integers.masked` now requires native artifacts, and the complete fixture passes.
Move (71 jobs) and Rust (135 jobs) pass; IR retains only the same seven legacy
Performance regressions. The latest full Check audit remains the preceding
constant/scalar-call **60/61** audit; no new full audit is claimed for this step.
Shift computation/agreement laws are the next worktree step below. No caps, baselines,
source-port counts, or checkpoint status changed.

Twelfth migration step (worktree): twelve positive native shift proofs, two
rejected contracts with artifact rollback, and seven exact executions pass. The shared typed shift
operation preserves wrapping, signed extension, mixed-width count operands,
and invalid-distance abort payloads. `Integers.shifted` and `Integers.halved`
now explicitly require native proofs; the complete Integers fixture passes.
Constant shift distances must be normalized consistently in result bounds and
continuation guards: the arithmetic solver otherwise treats `Int.toNat 1` and
`1` as distinct arguments of an opaque shift expression.

Profiling located the continuation cost in kernel checking `nativeSummary`,
not execution agreement or tactic search. The shared right-shift lemma now
provides the bound `2^(width-distance)-1` directly; generated VCs do not also
carry a redundant division equation. Typed local results retain a value
equation across modular calls, with constant distances normalized consistently
at the specification boundary. `local_shift` falls from 46.57M to 3.36M raw
heartbeats, and shift/add/call from 30.63M to 5.55M. Both shift-call tests check
that their proofs reuse the callee's native summary.

All twelve targets cost 2.51–5.55M raw heartbeats and 3,667–7,585 proof objects,
counting every generated stage. Their regression gate is now tighter: 10M
aggregate heartbeats and 15,000 aggregate objects, with a separate 1M cap for
each exact execution assertion. No existing baseline or budget was raised.
IR passes apart from the same seven legacy Performance regressions; Move
(71 jobs), Rust (137 jobs), and the five affected acceptance fixtures pass.
The fresh full Check audit passes **60/61**, with only the already-known
`Language/Generics.choose` budget failure. Vector construction/length migration
follows below.

Thirteenth migration step (worktree): bounded native vector construction,
nesting, parameters/locals, nominal fields, modular calls, and length. Constructors
use the shared typed operand product, including effectful and call operands;
length uses the stored native array bound. Separate agreement proves the exact
primitive result. Callee facts propagate array sizes without decoding elements
or unfolding callee computations. Generated specification bodies now explicitly
inject logical scalar call results at physical vector/field boundaries, matching
existing local-value behavior; executable types and runtime arithmetic are unchanged.

Seventeen positive proofs, two rejected contracts with complete artifact rollback,
six summary-reuse guards, and seven exact native executions pass. All generated
stages cost 1.13–6.97M raw heartbeats and 1,812–8,298 objects; the registered suite
enforces 10M/15,000 aggregate gates and a separate 1M cap per exact execution.
`Language/Vectors.make` and `length` explicitly select native and require its
artifacts. Before the native-only cutover, the four reference consumers still
used `compose`; they now fail closed pending native implementation. At that
pre-cutover checkpoint, IR functional/native cost checks, Move (71 jobs), Rust
(140 jobs), and **60/61** Check files passed, with the same seven legacy
Performance regressions and the known `Generics.choose` timeout. This is historical
evidence, not a passing native-only gate. No existing caps, baselines, or
source-port counts were changed.

Whole-file migration audits are now enforced by `#leaner_require_native_all`
in the four fully migrated acceptance fixtures above (excluding Integers,
whose reference target fails closed), and in native constant/bitwise/shift
unit fixtures, plus the new native vector suite. The route regression rejects
retired routes before cache reuse and rejects a malformed cached theorem in
both the per-function audit and the whole-file audit. Historical source ledger
rows are labeled **source checkpoint**, not completed native ports.

Fourteenth migration step (worktree, fallback disabled): shared-reference reads
use native scalar, Boolean, vector, and monomorphic nominal values. Copying
preserves the operand computation exactly; the new generic copy-agreement law
adds no native bind or runtime decoding, including for effectful operands.
Immutable local borrows have a separate agreement law retaining slot/kind checks
without minting loans. Shared arguments reuse native callee summaries. Boolean
reference specification binders now use their referent's logical domain; exact
arithmetic abort payloads normalize at leaves before arithmetic closing.

The registered `NativeReferences` suite passes fifteen positive proofs, two
false-contract rejections with artifact rollback, two summary-reuse guards,
seven exact computation/execution assertions, and a whole-file native audit.
All generated stages cost 1.04–5.88M raw heartbeats and 1,732–5,700 objects,
under new 10M/15,000 guards and separate 1M exact-execution caps. The original
`Language/VectorOperations.borrowed_length` passes its explicit native audit;
the rest of that fixture is not promoted. The final native-only Check audit
improves to **18/61**: `Language/Integers` passes all twelve verified targets,
native-artifact audits, and interpreter assertions. IR retains the same six
failing targets; native regression/cost gates, Move (71 jobs), and Rust
(141 jobs) pass. Mutation/reborrows, indexing, typed storage, and loops remain
incomplete; no existing caps or performance baselines were changed.

Fifteenth migration step (worktree, fallback disabled): checked shared vector
indexing uses one native array read guarded by the source bounds check. The
generic checked-value agreement law preserves normal, throwing, and undefined
behavior; exact indexed-borrow laws retain reference-kind, slot, and index
premises. Literal, dynamic, computed, nested, and shared-reference indices are
supported, including `u8` elements with `u64` indices. Nested initializers
reassociate without changing evaluation order. An overflowing index expression
throws its arithmetic error before the vector check; invalid vector indices
retain Move's `runtime.vector_error` and payload `1`, not a library abort code.

The registered `NativeIndex` suite passes thirteen positive proofs, three
rejections with full artifact rollback, two callee-summary reuse guards, eight
exact result/error assertions, and the whole-file native audit. Costs are
2.04–7.21M raw heartbeats and 2,371–7,520 objects across all generated stages,
under unchanged 10M/15,000 guards and separate 1M exact-assertion caps. Caller
length premises normalize selectively. An indexed consumer projects the
producer's encoded-array contract fact once at the requested index; neither
callee computations nor runtime vector decoders are unfolded.

Seven existing targets gain explicit passing native audits: `Vectors.middle`,
`VectorOperations.nested`, `bool_round_trip`, `read_out_of_bounds`,
`ControlForms.index_arithmetic`, and `VectorBounds.read_bad`/`read_reference`.
Their containing fixtures still fail, so the fresh complete Check audit stays
at **18/61**. The same six IR targets fail; native regression/cost gates, Move
(71 jobs), and Rust (142 jobs) pass. Mutable indexing, assignments, storage,
and loops remain incomplete; no source-port count or existing baseline changed.

Sixteenth migration step (2026-09-08, worktree, fallback disabled): straight-line
local assignments become typed rebindings. Generic block/assignment agreement
laws retain slot bounds, evaluation order, and abrupt control. Pure aliases
propagate native values directly; rebindings use distinct names and invalidate
only affected variant facts. The agreement explicitly supplies the known local
layout instead of leaving nested slot updates for Lean to reduce repeatedly.
The swap regression falls from exceeding 50M heartbeats to 3.24M; these frames
exist only in execution agreement, not in native computations or VCs.

The checked arithmetic emitter now supports multiplication with effectful
operands and local assignments. Unsigned products retain a direct nonnegative
proof from their typed operands. Abort payload normalization is shared with the
existing scalar shape. Named Boolean equations close truth-value equivalences
before splitting; constant negations reduce before contradictory local facts
can become looping simplifier rules.

The registered `NativeAssignments` suite passes twelve positive proofs, two
rejections with complete artifact rollback, a callee-summary reuse guard, six
exact result/error assertions, and a whole-file native audit. Coverage includes
repeated writes, saved values, swapping, Booleans, nominal values, vectors,
callee-initialized assignments, and symbolic/nested multiplication. Costs are
1.78–6.03M raw heartbeats and 2,172–6,752 objects under unchanged 10M/15,000
guards; exact assertions and a Boolean-closer regression have 1M caps.
`ControlForms.compound_local` passes an explicit native audit. Its fixture still
fails, and the fresh complete Check audit remains **18/61**. The same six IR
targets fail; dedicated native-computation regression/cost gates, Move (71 jobs),
and Rust (143 jobs) pass. Statement-position branches, unit-valued effects,
mutable references, typed storage, and loops remain incomplete. No existing
budget or baseline was changed.

Seventeenth migration step (2026-09-08, worktree, fallback disabled): a branch
whose arms assign the same local is hoisted into a typed choice followed by one
continuation. The generic agreement laws retain abrupt control, intermediate
states, and assignment bounds, and rewrite only the statement head. Nested
branches, checked arithmetic, calls, nominal values, and vectors are supported.
Multi-local joins and the other statement-control forms remain incomplete.

Pure integer choices join as one native value; arithmetic projection rules
expose the selected integer without duplicating the continuation proof into
every pure arm. Effectful results keep their equations and bounds. Branch-local
calls propagate call-use metadata to preceding arithmetic. No callee body is
reverified, and no runtime frame enters a native computation or VC.

The registered `NativeConditionalAssignments` suite passes eleven positive
proofs, two rejections with full artifact rollback, a callee-summary reuse
guard, five exact execution assertions, and whole-file native auditing. Two
structural guards pin one copy of the arithmetic continuation and one join
around nested pure choices. All generated stages cost 2.58–9.84M raw heartbeats
and 3,000–11,631 objects under unchanged 10M/15,000 guards; exact assertions
retain 1M caps. Repeated branches improve from 12.04M to 9.24M. Failed range-
pruning and frame-collector experiments were discarded, not left in production.

`ControlForms.clamp` and `then_else` now pass explicit native audits without
promoting their still-incomplete fixture. The fresh complete native-only audit
remains **18/61**, with the same 41 native-gap files and two diagnostic-only
mismatches; no previously passing file regressed. The same six IR targets fail; all
dedicated native regression/cost gates, Move (71 jobs), and Rust (143 jobs)
pass. Unit-valued effects and early loop control remain prerequisites for
native loop consumers. No existing budget, baseline, or source-port count was
changed, and the legacy fallback remains disabled.

Eighteenth migration step (2026-09-08, worktree, fallback disabled): Unit
results/locals, discarded call results, nested Unit blocks, statement-position
assertions, and explicit aborts with computed u64 codes use native computation.
General discard/throw agreement laws retain operand order, intermediate
states, and exact failure payloads. Unit calls use the callee's native summary;
a local no-abort normalization removes argument-equality witnesses without
reopening its body. Unsupported embedded `spec` assertions remain fail-closed,
not silently erased with their runtime no-op.

`NativeStatements` passes fifteen positive proofs, four rejected cases with
artifact rollback, four modular-summary reuse guards, eight exact execution
assertions, and whole-file native auditing. Tests distinguish first failure,
division's empty payload, and overflow's computed payload. Aggregate costs
are 0.75–6.37M raw heartbeats and 1,470–7,462 objects under 10M/15,000 guards.
The existing conditional-call cost remains under its tight 10M cap (9.923M).

`ControlForms.checked_assert` and six state-free `Verification/Aborts` targets
now have explicit native audits; their whole fixtures remain incomplete.
The fresh native-only audit remains **18/61**, with the identical pass list,
41 native-gap files, and two diagnostic-only mismatches. The same six IR
targets fail; dedicated native regression/cost gates, Move (71 jobs), and Rust
(144 jobs) pass. Typed loop translation/invariants, multi-local joins, Unit
arguments, mutable references, and storage remain unfinished. No fallback,
existing baseline, budget, or source-port count changed.

Native loop kernel checkpoint (2026-09-08, worktree; no source port claimed):
`NativeLoop.run` is a typed-state/typed-exit fixed point with exact success,
failure, and undefined-execution characterizations. Its invariant rule checks
entry and one iteration, not an unrolled iteration count. Both directions of
agreement with the structured execution loop are proved, including failures
after intermediate state changes. Normal values and unlabeled continues repeat;
an unlabeled valueless break exits. Unsupported outer control/returns are
excluded by the certificate, not silently discarded. The owned-local law
passes one typed exit state into one continuation. Runtime frames remain only
at execution agreement.

The registered `LeanerIR.Tests.NativeLoop` covers arbitrary countdowns,
store-changing invariants, later abort payloads, undefined behavior, divergence,
concrete local assignment/break, continue, and explicit outer-control rejection
under 1M-heartbeat declaration caps. False invariants cannot erase executions.
Thirteen axiom audits and native-representation guards pass. The full IR gate
retains the same six failures. The source generator and authored invariant
binding are still pending; no acceptance target or native-audit count is
promoted by this kernel checkpoint.

Nineteenth migration step (2026-09-08, worktree, fallback disabled): source
loops now use that native fixed point. Header locals are heterogeneous products
of typed integers/Booleans. Authored predicates read those values directly;
the unused frame/existential invariant builder and its `loopSlotValue` helper
have been removed. Unannotated loops retain immutable header values and typed
integer bounds. Non-loop `spec` assertions are never erased as loop annotations.

`NativeFlow` shares typed statement joins while preserving normal, continue,
and break control. Separate agreement laws retain exact final frames, stores,
abort payloads, and undefined behavior. The WP proves entry and one iteration;
calls reuse native summaries, and sequential loops each have one fixed point.
The registered `NativeLoops` suite passes twelve positive proofs, four
rejection/rollback checks, five exact generated-computation executions,
representation/admission audits, and structural no-duplication guards. All
generated stages, including invariant declarations, cost 0.987–10.305M raw
heartbeats and 2,139–10,199 objects under new 20M/30,000 guards. Exact execution
tests retain a 1M cap. Existing budgets and baselines are unchanged.

The fresh complete audit passes **19/61**, with `Verification/Loops` the only
new passing file. Three language-loop targets and two authored-invariant
targets pass explicit native audits inside incomplete fixtures. The six
known IR failures remain; native regression/cost gates, Move (71 jobs), and
Rust (150 jobs) pass. Body-local lifetimes, nested/labeled control and returns,
reference/aggregate/generic loop headers, and store-mutating loops remain
unfinished. This is source-semantic verification, not a compiler-correctness
claim about emitted bytecode.

Twentieth migration step (2026-09-08, worktree, fallback disabled): lexical
body locals no longer have to become permanent loop state. `NativeFlow`
distinguishes the ordinary join row from the enclosing loop's control row.
Normal paths can therefore retain a temporary across a branch join, while
break/continue carry only the header locals. `Observed` agreement relates
those live values to execution frames in both directions; completeness
constructs a real reachable frame, not an arbitrary frame satisfying the
relation. Dead typed optional cells occur only in agreement witnesses, never
in computations or VCs. A shared linear cell proof checks that discarded
cells contain no loans. Exact intermediate stores and failures are retained.

The registered `NativeLoopLocals` suite passes twelve positive proofs, two
negative/rollback checks (including invariant-metadata rollback), four exact
native executions, four explicit law-admission checks, a modular-summary reuse
guard, and structural no-duplication/dead-cell guards. Coverage includes
shadowing, temporaries across branch joins, branch-scoped locals, continue,
early break, Boolean/Unit temporaries, temporary-derived abort payloads,
sequential loops with an authored invariant, and a modular call. Pure integer
aliases expose their value equation locally so subsequent invariant VCs can
reuse arithmetic facts. No callee body is reopened. Aggregate costs are
1.260–11.529M raw heartbeats and 2,763–12,761 objects under new 20M/30,000
guards; exact execution tests use 1M caps. The tight existing conditional-call
case still costs 9.923M under its unchanged 10M cap.

Both `Language/Loops` shadowing targets now pass explicit native audits. The
fresh complete Check audit remains **19/61**, with exactly the previous pass
list: 39 native-gap files and three diagnostic mismatches. The same six IR
targets fail; native regression/cost gates, Move (71 jobs), and Rust (154 jobs)
pass. Nested/labeled control and returns, reference/aggregate/generic loop
headers and body locals, and store effects remain unfinished. No fallback,
existing baseline, budget, or historical source-port count changed.

Twenty-first migration step (2026-09-08, worktree, fallback disabled): nested
and labeled loop control now carries a heterogeneous sum of enclosing typed
headers. Each loop consumes its own control and passes outer control outward
without a runtime frame, universal value, or dead cell in the computation.
`ControlRoute` encodes control only in execution agreement;
`observed_nested_loop` proves both simulation directions, exact intermediate
stores and aborts, and absence of undefined executions. `observed_map` forgets
lexical locals at normal joins while preserving reachable frames. The existing
compact single-loop path is retained, with a proved root-control conversion.

The registered `NativeNestedLoops` suite passes nine positive proofs, two full
artifact/metadata rollback cases, four exact native executions under 1M caps,
four law-admission checks, and structural no-frame/dead-cell/duplication guards.
Coverage includes three-level labeled break/continue, heterogeneous inner/outer
headers, temporary-bearing normal exits, nested abort payloads, and an authored
inner-loop invariant. Aggregate costs are 2.409–17.231M raw heartbeats and
4,678–13,630 objects under new 20M/30,000 guards. Existing native performance
budgets remain unchanged; the tight conditional-call case remains at 9.923M.

The three labeled `Language/Loops` targets now pass explicit native audits
(eight audited targets total in that incomplete fixture). The remaining
verification errors there are `drain` and `early`. The fresh complete Check
audit remains **19/61**, with the identical pass list, 39 native-gap files,
and three diagnostic mismatches. The same six IR targets
fail; native regression/cost gates, Move (71 jobs), and Rust (158 jobs) pass.
Early-return source generation/transport is next; reference/aggregate/generic
loop state and storage effects are still unfinished. No fallback, baseline,
existing budget, or historical source-port count changed.

Twenty-second migration step (2026-09-08, worktree, fallback disabled):
statement-position early returns now carry a native result at the bottom of
the nested-loop target sum. Only normal control executes the shared
continuation; checked return operands still propagate exact abort payloads.
`controlled_return`, `operands_single`, and `fromFrame_returning_discard`
provide the execution transport and preserve actual function-boundary loan
export. Frames and return encodings remain agreement-only. Non-returning
paths keep their existing emitters.

The registered `NativeEarlyReturns` suite passes eleven positive proofs, two
complete artifact/invariant-metadata rollback cases, nine exact native
executions under 1M caps, three law-admission checks, a callee-summary reuse
guard, and structural no-frame/dead-cell/duplicated-continuation guards.
It covers scalar/Boolean/Unit results, overflow in return operands, normal
joins, loop-local and nested-loop returns, skipped aborts, and modular calls.
Aggregate cost is 2.297–11.475M raw heartbeats and 4,351–12,365 objects under
new 20M/30,000 guards. Pure returned values feed the postcondition directly.
Existing native budgets are unchanged; the tight conditional call is 9.923M.

The source frontend previously erased a return at the tail of any nested
block, including a loop body. Conversion to a fallthrough value is now
restricted to genuine function-tail positions. Unit returns carry no results
but still evaluate their operand. `Language/Loops` now audits `early` and
`return_in_loop` (ten v0 targets total), and adds two native scope regressions
with three source-interpreter executions. Only `drain` remains a verification
gap in that fixture. The fresh complete Check audit remains **19/61**, with
the identical pass list, 39 native-gap files, and three diagnostic mismatches.
The same six IR targets fail; native regression/cost
gates, Move (71 jobs), and Rust (160 jobs) pass. General value-position abrupt
expressions, returned references, reference/aggregate/generic loop state, and
typed storage effects remain unfinished. No fallback, baseline, existing
budget, or historical source-port count changed.

Twenty-third migration step (2026-09-08, worktree, fallback disabled): native
mutation prerequisites, **not a completed source port**. `NativeVector` now
provides bounded native replacement and checked element assignment with
size/selected-element/unrelated-element laws and an exact projection-write
agreement. `NativeMutation` updates a live reference's typed current value
without changing its loan identity. Its agreement includes the interpreter's
registered reference-mutation evaluator at an arbitrary slot, plus exact
checked dereference/index write, failure, and definedness laws. Native updates
contain no frame, decoder, or runtime-value payload representation.

`NativeVectorMutation` is registered in the IR suite. It checks symbolic length,
element and loan preservation, negative/upper/empty-vector bounds, wrong-output
and wrong-loan rejection, actual registered mutation with untouched sibling
locals, and exact zero/three-iteration clearing. A native aggregate clearing
loop proves every bit false and preserves length and its unrelated field with
one invariant step, independent of vector length. Test declarations and the
new mutation modules use a 1M-heartbeat cap; selected proof terms have a 10k
object cap. The new cap exposed costly commutative proposition sorting in a
write agreement; direct conjunction transport removed that work.

The existing `fromFrame_loop` agreement now threads the actual exit store into
normal/aborting continuations without a state-preserving-iteration premise.
`NativeSequence` uses the generalized law; regression tests exhibit an
effectful iteration and a continuation observing its changed allocation count.
The fresh Check audit has the identical **19/61** pass list; native cost gates,
Move (71 jobs), and Rust (160 jobs) pass, with the same six IR failures.
`clear` and `drain` still need source borrow/reborrow and native loan-export
integration. No source fixture is promoted, and no existing cap or baseline
changed. Current per-file status lives in `test-organization.md`; detailed
checkpoint history is now separate in `test-organization-history.md`.

Twenty-fourth migration step (2026-09-08, worktree, fallback disabled): checked
ownership-output boundaries. `NativeBoundary.finish` projects a native output
and commits its owners once, on successful completion, using the actual middle
store. Aborts and undefinedness are unchanged. `Transports` independently proves
that the native contract entails the source contract after that commit, retaining
unconditional frame and must-abort obligations. `fromFrame_boundary` connects
the actual final execution frame and returned values to that adapter. The
frame-changing `nativeOperation_update` law now compiles under its 1M cap.

The public verifier accepts this transport only with both the exact execution
agreement and a checked `computationBoundary` contract bridge. Native body/VC
audits forbid the boundary adapter and certificate as dependencies, so loan
export does not become a native operation. Generic boundary transport currently
rejects; the ordinary native route is unchanged. No legacy fallback is enabled.

The registered `NativeMutableBoundary` pilot supplies independent typed owner,
native contract, and agreement artifacts for authored `set_seven(&mut u64)`.
Public verification, native audit, cached reuse, exact source-denotation
write-back, and missing-write-back rejection pass. A malformed bridge rejects
without publishing generated artifacts. All six proof stages total 2.36M raw
heartbeats / 3,336 objects, within the existing reference-test 10M/15k ceiling.
The registered IR boundary regressions check actual changed-store completion,
aborts, undefinedness, wrong finalizers/postconditions, and unconditional frame
and must-abort checks, with 1M declaration and selected 10k-object proof guards.

This is an explicit source pilot, not automatic mutable-body generation or a
completed Check-file port. Source reborrow lifetimes, owner-returning callee
summaries, and `clear`/`drain` integration remain. The fresh capped audit has
the identical **19/61** pass list. All registered IR/native roots except the
same six failing targets pass (198 jobs); Move (71 jobs) and Rust (161 jobs)
pass. No existing budget or baseline changed. Current status and evidence are
in `test-organization.md`, with detailed chronology in its history archive.

Twenty-fifth migration step (2026-09-08, worktree, fallback disabled): automatic
scalar-owner mutation. `NativeMutable` recognizes the actual reference-mutation
operation with one fixed-width integer parameter and a literal replacement,
not source/test names. `Contract.buildNativeOwnerContract` separately translates
authored postconditions over the updated typed owner and `old` over its entry
value. Conditional abort codes and partial/strict/unspecified abort semantics
match the source contract. The native VC proves those authored predicates;
the separate boundary only transports the proved facts and exports the loan.
Generation is synchronous under the existing verification budget, and any
proof-stage error prevents publication of native source theorems.

Registered `NativeMutableGenerated` has nine positive targets, three false
contract rejections, five execution/result/loan assertions, admission checks,
and dependency guards over the computation and native postcondition. It covers
signed/unsigned widths, old values, conditional abort codes, partial/strict/
unspecified abort behavior, and no postcondition. All generated stages cost
3.28–4.23M raw heartbeats / at most 5,384 objects, within the existing reference
10M/15k ceiling. `NativeComputationRoutes` now verifies its direct writer and
retains an explicit unsupported-reborrow regression instead of treating the
working writer as an expected failure.

Checked replacement expressions, additional locals, reborrows, generic owners,
and owner-returning native call summaries remain next; no failing Check file
is promoted. The fresh full audit has the identical **19/61** pass list. All
registered IR/native roots except the same six failing targets pass (200 jobs);
Move (71 jobs) and Rust (165 jobs) pass. The tight prepared-call regression
remains 9,923,105 heartbeats / 11,631 objects. No existing cap or baseline changed.

### Earlier normalization status (not full native migration)

Implemented for the supported corpus, updated 2026-09-06. The
normalization route now includes modular calls, recursive fixed points,
typed generics, and loop invariants; these are no longer blanket fallback
categories. It supersedes the per-shape route scripts of
`certifying-execution.md` (N1–N4 and the ports after them). Remaining feature
combinations and acceptance ports are tracked in `test-organization.md`;
the checkpoint sections below record the rollout and its performance gaps.

## Why

The frame-free row route was built to remove the frame representation's
20–50× cost, and it did. But it was built as *generated tactic scripts
matched to body shapes*: `LeanerLang/RowScript.lean` holds 22 recognizers
and 20 script emitters, 6.8k lines, over ~200 hand-stated row laws, many
one per slot layout (four `mutate_evaluate_rowFrame_*`, seven
`exportFrameLoans_rowFrame_*`, bracket laws with a four-slot row written
out). A plan covers the function it was written for and its neighbours;
plans do not compose, so every ported v0 file costs a plan and the corpus
is unbounded. v0 verified 176 targets, 99 automatically, with a 1.2k-line
WP calculus and 0.5k lines of tactics. The leaner stack has 26k lines of
verification code and 50 targets.

There are two regressions, and the first draft of this design removed
only one of them.

1. **Shape matching instead of composition.** Fixed by having one law per
   combinator and no recognizer. Any design does this.
2. **Stepping instead of reduction.** The leaner denotation is
   *relational*: `ExprDenotation` is a `Prop` over frames, states, and
   controls. Verifying it means a tactic steps it — apply a law per node,
   receive an evaluator *equation* as a hypothesis, invert it by
   `injection` and `subst`, reduce a `match`, split, continue. That is
   proof-producing symbolic execution: a cost per step, a dependence on
   the goal's syntax at every step, and the machinery of `leaner_row_drive`,
   `leaner_row_head`, the inversion chains, and the hypothesis-naming
   conventions the scripts spell out. v0 had none of it. Its `wp (program)
   post` was a term that *normalized*: `@[wp_norm]` iff-lemmas, one per
   combinator, rewritten by `simp`, with arithmetic such as
   `Checked.addSpec` reducing to its range condition. The verification
   condition was computed by rewriting and only the residue reached the
   closer. Composition came from `bind`. That is the advantage identified
   in `historical/verification-v2.md`, and the reason v0 was sub-second and
   shape-free.

V1 chose relations over frames so that agreement with the big-step
relation would be exact and direct. That was the right form for
*agreement*. It was the wrong form for *verification*, and the row route
then built an executor on it instead of a normal form. This design keeps
the relational denotation for agreement and adds the computational one
for verification.

## The principle

```
verify f  =  normalize ; close
```

- A body denotes, besides its relation, as a **computation**: a `Spec`
  over a symbolic state — the row, the registries, the runtime state —
  returning a control. `Spec` and its `wp` exist (`Proofs/Spec.lean`,
  `Proofs/Contract.lean`, the `lir_wp_norm` simp set); the typed contract
  layer already speaks `wp`.
- Every combinator has one **`wp` iff-lemma**, `@[lir_wp_norm]`, that
  rewrites `wp (C …) post` into `wp` of its parts, and every leaf
  operation has an **evaluation lemma** that reduces its closed evaluator
  on a literal row to a value or a throw with its side condition.
- **Verification** is `simp only [lir_wp_norm, leaner_eval]` followed by
  `leaner_certified_close!`. No drive, no plan, no inversion, no names.
- **Agreement** between the computation and the relation is one lemma per
  combinator, composed by the generator exactly as V1 composes the
  relation's agreement with the big-step semantics. The chain from
  `verified` to `functionSpec` stays exact: the transport through
  `Spec.Equiv` is unchanged.

The generator therefore emits three things per function: the relation
(unchanged), the computation, and the computation's agreement with the
relation; and one tactic, `leaner_normalize`, for the theorem. A body the
normal form cannot finish leaves a residue at a program point, reported
by the closer at the clause it could not establish — the same visibility
as today, at the granularity of a combinator.

## The computational denotation

```lean
/-- The symbolic state a body runs over. -/
structure RowState where
  row : Row
  registries : Registries
  state : RuntimeState

/-- A body as a computation: a control, or a throw. -/
abbrev RowSpec := Spec RowState Failure Control
```

with combinators mirroring `ExprDenotation`'s: `pureValue v`, `readLocal i`,
`operation ev operands` (evaluate the closed evaluator on the operands'
values; a throw is `abort`), `branch c t e`, `letIn i init body`,
`assign i v`, `loop site body` (as `Spec.withInvariant`, the fixed point
the loop laws already use at the contract level), `throw_ k args`,
`return_ vs`, `call handle callee args`, `globalBorrow key path`,
`endLoan loans`. Operand rows and statement rows are `bind`s. Nothing
carries a `RuntimeFrame`; the row is a literal list and the state is a
record updated structurally, which is what made the row route's
representation cheap and is kept.

Each combinator's `wp` lemma is an iff in the form v0's were:

```lean
@[lir_wp_norm] theorem wp_operation …
    wp (operation ev operands) post s ↔
      wp operands (fun values s' =>
        match evaluate ev values s' with
        | .value v s'' => post (.value v) s''
        | .throw k a s'' => post (.throw_ k a) s''
        | none => False) s
```

and the evaluation lemmas (`leaner_eval`: the existing `*_evaluate` laws,
`readLocal?_rowFrame`, `set!` normalization, the family representation
facts) reduce `evaluate ev values s'` on literal data so the `match`
reduces. A checked operation leaves its range condition; a global read
leaves the family lookup the closer already resolves through
`Representation.lean`. Where an operation's evaluator is stated on a
`RuntimeFrame` today, the row form is the evaluator at `rowFrame row
registries`, and the lemma is proved once from the frame lemma.

Controls are a sum in the result, so `bind` sequencing dispatches on them
once, in `wp_bind` for statement rows: a non-value control propagates
without evaluating the suffix, which is the block rule.

## Calls, storage, and references as `bind`s

- **Calls.** `call handle callee args` is `bind` of the argument row into
  the callee's `Spec` at the argument values; `wp_call` consumes the
  callee's `Satisfies` fact (`wpFunction_of_satisfies`, exists) so the
  continuation runs with the callee's `ensures`, `frame`, and `aborts`
  clauses as hypotheses. The contract is data: `Contract.runtime` over
  `typed` over the raw contract unfolds by simp to existentials and
  conjunctions, named by shape by the same pass that names the caller's
  precondition (`leaner_native_cases`, `leaner_name_facts`). The recursive
  call is the same lemma with the fact taken from the induction
  hypothesis; nothing else about recursion is special.
- **The `&mut` write-back** is the prophecy pair in the state: the
  callee's summary says `pending = push (loan, newValue)`; `endLoan`
  resolves it into the row at the borrow's slot, one lemma parametric in
  the slot (`resolveReturnedBorrows`), replacing the reborrow-call laws
  that fixed the slot at 0.
- **Storage.** A bracket is `globalBorrow`, the body, and `endLoan`. The
  borrow's evaluation lemma produces the borrow value, registers the loan
  at `.global key`, marks the hole in the family, and bumps `nextLoan`,
  parametric in key and path; take, publish, and contains are each one
  lemma; the global `endLoan` writes the borrow's current value back to
  the family. Field focus is a path in the loan's location and a
  `focusValue` in the hole, already the runtime's representation. The
  bracket laws written out over a four-slot row are their composition.
- **Returned references** are values in the row plus the summary's export
  clause; the caller's `call` registers the returned loan, the callee's
  exit exports it (`exportFrameLoans_rowFrame_holeFree`, exists, with its
  side condition decided by simp on the literal row).

## The pilot

Before any of this is generalized, it is built for **one hard target and
benchmarked against its script**. The target is `withdraw`
(`perf_storage`, the `Account` port): a storage bracket with a focused
field, a `let`, a comparison and branch, an abort, checked subtraction,
a write through the borrow, and the loan's death — the shape whose script
laws are the most hand-written (`wpRowThrow_focusedFieldBracket` and its
variants), and the target that carried the original 51× finding
(379.9M heartbeats on the frame route, 17.3M under the script today,
48k proof objects).

The pilot builds exactly the combinators `withdraw` needs, their `wp`
lemmas, their evaluation lemmas, their agreement with the relational
combinators, the generator's emission of the computation for that body,
and `leaner_normalize`; then runs `verify withdraw` through it with the
script disabled. `deposit`, `replace`, and `balance_of` share its
combinators and are measured with it at no extra cost.

The pilot reports one table:

| target | route | heartbeats | objects | wall |
|---|---|---|---|---|
| `withdraw` | script | 17.3M | 48,197 | |
| `withdraw` | normal form (first cut, 2026-09-03) | 44.7M | 55,556 | ~1.1 s vs 0.55 s |
| `withdraw` | normal form (default route) | 39.7M | 60,018 | 1.28 s |
| `deposit` | script | 13.1M | 28,918 | |
| `deposit` | normal form | 31.5M | 45,682 | 1.02 s |
| `replace` | script | 8.5M | 24,912 | |
| `replace` | normal form | 17.1M | 24,196 | 0.55 s |
| `balance_of` | script | 11.9M | 16,523 | |
| `balance_of` | normal form | 11.7M | 12,352 | 0.35 s |

### Pilot result (2026-09-03, scratch `PilotWithdraw.lean`)

`withdraw` verifies by normalization: the hand-mirrored computation
`withdrawC`, its agreement and totality lemmas (3.2M heartbeats, 752
objects, generated per function later), then in the theorem two `simp`
passes over the `wp` lemmas with the ground-evaluation simprocs, the
contract prefix, and `leaner_certified_close!` on the three residual
obligations. The residue after normalization is v0's shape: the abort
implication, the normal path with the final state as a keyed insert of
the written resource, and the arithmetically impossible overflow branch.
No per-shape law and no drive is involved.

The cost, unoptimized, splits as (heartbeats):

| step | cost | note |
|---|---|---|
| prologue (`leaner_cases`, representation, present resource) | 3.3M | shared with the script |
| switch to the computation, entry facts | 2.1M | |
| normalization, first pass (`wp` lemmas, reads) | 3.2M | |
| normalization, second pass (evaluators, write-back, export) | 16.0M | 11.4M in the four ground evaluator runs: `endLoans?` twice at 4.3M, `mutateBorrow?` 1.8M, `exportFrameLoans` 1.0M |
| closing, abort branch | 6.5M | the closer alone |
| closing, normal path | 2.7M | 1.1M of it the fits certificate and the representation fact |
| closing, overflow branch | 0.5M | `exfalso; omega` once the resource's bound is exposed; 13.4M through the closer |
| kernel check | ~6.5M | the 55k-object term |

Findings, in the order they will be worked:

1. **Ground evaluation is the cost center.** Each evaluator run is an
   inner `simp` with the whole ambient rule set plus the unfolding set;
   it needs the local focus and loan facts (a run with only the default
   set stalls), so the fix is a dedicated evaluation set carrying just
   those facts, or reduction with a reflexivity proof. Target: the four
   runs under 3M together.
2. **Unreachable branches must be pruned in normalization**, by an
   arithmetic discharger on the range conditions, with the resource
   bounds exposed as plain facts at the prologue. The closer is 25×
   more expensive than `omega` on such a branch.
3. **The closer needs the written resource as a twin.** The
   representation fact is stated in the goal's `focusValue` spelling and
   proved by the twin's `insert_over_hole`; the generator must emit that
   fact from the write it sees in the normal form (the fits certificate
   from the range condition). The closer's key normalization keys on
   `RuntimeValue.storageKey`, so the prefix must rewrite `globalKey` by
   `rfl`, never unfold `storageKey`.
4. **Clearing the context does not help**: the closer's cost is in the
   goal, not the hypotheses.

The decision: the route is correct on the hardest scalar-storage target
and its cost is 2.6× the script's heartbeats and about 2× its wall time
at a first cut whose profile names the three fixes above. That is within
the range the design allows for the pilot; the general build-out
proceeds, with finding 1 fixed inside P1 before `deposit`, `replace`,
and `balance_of` are measured.

### Build-out state (2026-09-03)

The route is in the codebase and `leaner.route` now defaults to
`normalize`. `verify` uses it for supported, non-recursive, non-generic
trees and automatically retains the script route for the remaining
categories.

**Modules.**

- `LeanerIR/Proofs/Tree.lean`: the body as a literal tree (`Tree`,
  `Operands`, `Statements`), `denote` (the relational combinators),
  `compute` (the `RowSpec` combinators), `supported`, and agreement and
  totality **proved once by mutual induction**; `wp_nativeFunction_tree`
  is the one-rewrite switch. The generator
  (`LeanerLang/Denotation.lean`, one traversal `emitWith` over
  `relationalBuilder`/`treeBuilder`) emits `<f>.denotationTree` beside
  the body and `denotationTree_denotes` by `rfl`; `Generated` carries
  `tree`/`treeDenotes`. No function carries an agreement proof.
- `LeanerIR/Proofs/Normalize.lean`: the `lir_eval` set; `wp_evaluate`,
  `wp_ite_prop`, the checked add/sub/mul/div/mod and modular add/sub/mul
  lemmas keyed on the primitive (`[lir_wp_norm high]`: they must beat the
  generic `wp_evaluate`); ground-evaluation simprocs, one per evaluator
  head, unfolding the evaluator's own definition closure
  (`unfoldClosure`; the abstract `GlobalMap` and `globalLoanKeyIn?` stay
  folded) at literal rows; `evalInitialLocals` builds the entry row;
  `evalBind` for struct patterns; `reduceBEq`/`reduceBne` decide closed
  derived-`BEq` comparisons; `decideLoanEquality` decides `Nat`
  equalities on loan ids by `omega` (negative results cached);
  constructor equations for the well-founded walkers (`collectPruned_*`,
  `findFirst_*`, `rewriteFirst_*`), the registry (`globalLoanKeyIn?_cons_ne`),
  and the returned-borrow resolution; `leaner_normalize [facts]`, which
  also reads every context fact about `initial` (equations, freshness,
  loan bounds and distinctness in both orientations). The outer simp no
  longer runs `omega` as a catch-all discharger; arithmetic is invoked by
  the keyed rules and closer that need it.
- `LeanerIR/Proofs/Decode.lean`: `evaluateDecoding` — a twin's `decode?`
  (or a codec's) evaluated on a literal, its range certificates proved
  from the context, returning the twin and the proof.
- `LeanerIR/Proofs/Represent.lean`: `leaner_expose_resources [tree]`
  (splits the typed contents at every keyed read of the tree into
  absent/present; a requires-side presence fact closes the absent case;
  `leaner_certify!` destructures the twin; `leaner_split_bools`
  enumerates Boolean fields), `leaner_represent_writes` (every
  `insert`/insert-over-hole/`erase` of a nominal literal in the goal gets
  its `FamilyRepresentation` fact, stated in the goal's spelling),
  `leaner_decode_results` (instantiates `∃ result, decode literal = some
  result ∧ …`, codec results included), `leaner_close_normalized`.
- `LeanerIR/Proofs/Certify.lean` (shared closer): `decodeLiteralTwins`
  before an equation's rows; literal-nominal rows in
  `leaner_resolve_rows` (`resolveReturnedBorrows_singleBorrow`, the fill
  laws in both spellings); the arithmetic leaf accepts a goal its own
  rewrite closes (`<;> omega`).
- `verify` (`LeanerLang/Contract.lean`): on the route, for a supported,
  non-recursive, non-generic tree, one theorem shape — prologue, codec
  unfolding (twin codecs included), `leaner_expose_resources [tree] <;>
  (…)`, the switch, `leaner_normalize`, the contract prefix,
  `leaner_close_normalized`, `leaner_report`. Everything else stays on
  the script.
- `leaner-e2e-tests/…/Check/Verification/Normalized.lean`: the storage
  check under the route (CI coverage of the route).

**Coverage.** `lake env lean -D leaner.route=normalize` over every
positive `Check/Verification` file passes: Account, Callees, Calls,
Corpus, Generics (script fallback), Increment, Loops (script fallback),
Normalized, Prophecies, References, Rust, Storage, and Typed. The Aborts
negative exits nonzero and reproduces its expected diagnostics verbatim.

**Completed follow-up work.** The broad `subst_vars` close was replaced
by targeted substitution of decided Boolean variables. Profiling then
found the remaining multiplicative cost in two places: every ground
evaluator inherited the full ambient `lir_wp_norm` inventory, and the
outer normalization simp ran `omega` after every unsuccessful side
condition. Ground evaluation now uses only ordinary/local facts,
`lir_eval`, and the evaluator's definition closure, and the catch-all
arithmetic discharger is gone.

`exportFrameLoans` now has a generic structural one-borrow equation. Its
simproc computes only `frameBorrows`, the local-hole predicate, and the
global lookup, then constructs the proof from that equation; it does not
unfold the general settling algorithm into the proof term. This covers
scalar borrows and returned reborrows without a slot-layout-specific law.
The route figures are consequently back at the pilot scale: `withdraw`
39.7M, `deposit` 31.5M, `replace` 17.1M, `is_published` 9.9M,
`balance_of` 11.7M, and `reborrow` 9.7M heartbeats.

Decided Boolean parameters are substituted before normalization, so a
contract case normalizes only its selected branch. This brought
`choose_reborrow` back under the check suite's 50M-heartbeat cap; doing
the same substitution only in the closer was too late and normalized
both arms in each case.

The performance baseline was deliberately regenerated and reviewed, and
the default-route gate passes. The remaining work is P2/G2 coverage for
calls, recursion, returned-reference combinations, loops, and generics;
after those no longer select the fallback, G4 can remove `RowScript.lean`
and the per-layout laws.

**Traps met (so the next hand does not meet them again).** Inside
`namespace LeanerIR…` write `Lean.Expr`/`Lean.Name` (`LeanerIR.Expr`
shadows). Values in goals may be `mdata`-wrapped: `consumeMData` before
matching. `rootNamespace` is `_root_`: never `env.contains (rootNamespace ++ n)`.
A tactic that creates goals inside a `(a; b)` block needs `all_goals` or
`<;>` for the rest. Splitting the frame clause with `And.intro` unfolds
`LoanDiscipline` past the closer's grammar: let the closer take the
conjunction. The closer keys its frame hypothesis on
`RuntimeValue.storageKey`: rewrite `globalKey` by `rfl`, never unfold
`storageKey`. `set_option … in` is not accepted inside a `leaner module`
block. A `pkill -f` pattern that appears in the same shell command kills
the shell.

For future performance work, objects up means the goals carried more (a
state or row that stopped being consumed — a representation question in
the normal form); heartbeats up with objects flat means search (a simp
set too wide — a keying question). Either is fixed in the normal form
before expanding its coverage; neither is a reason to keep scripts.

The next coverage target is the call pair `bump_twice` and
`take_and_bump` (the write-back through a callee, 12.5M and 13.8M today),
because calls are the other axis on which the scripts fail to compose.

## Correctness gate

Every `verify` target that verifies today — the 15 benchmark targets and
every check under `leaner-e2e-tests/LeanerE2ETests/Check/` — verifies by
normalization with the scripts *disabled*. The two intended negatives keep
their diagnostics. No new `.exp` may appear. From then on a program shape
is never a reason to add code; only a missing lemma is.

## Performance gate

The scripts were the answer to a measured cost, so the normal form is
measured against them on the same targets before any script is deleted.

- **Instrument.** `Perf.measure` reports heartbeats and proof objects per
  generated theorem in `Performance.lean`. It is extended to the check
  suite: the driver records, per `verify`, heartbeats and objects of the
  typed theorem into a `.perf` file beside the check (regenerated with
  `UB=1`, compared at the same 10% tolerance), so the whole corpus is
  gated, not fifteen curated targets.
- **Two routes, one option.** During the migration the generator selects
  the route by `leaner.route := script | normalize`; the driver runs the
  corpus under both and prints the table above for every target.
- **Baseline.** The scripts' numbers, recorded in the appendix at P0.
- **Policy.** A regression is not a blocker; it is a number the milestone
  reports and explains. The gate that blocks is `Performance.exp` at its
  tolerance, moved deliberately with `UB=1` and the diff reviewed.

Expected sources of cost, to be watched: `simp` rewriting under the
binders `wp_bind` introduces (bounded by stating the lemmas so the
continuation is applied, not quantified, wherever the control is known);
state-record normalization (structural updates only, no `GlobalMap`
lookups computed at each step — the N3 finding stands); the naming pass
at each call (once per callee fact). The closer's share is unchanged and
is measured separately by the same instrument.

## Milestones

| | Step | Correctness gate | Perf gate |
|---|---|---|---|
| P0 | Measurement: per-check `.perf` records, `leaner.route`, the baseline table in the appendix | suites unchanged | baseline recorded |
| P1 | Pilot: `withdraw` by normalization — its combinators, `wp` and evaluation lemmas, agreement, emission, `leaner_normalize` | `withdraw`, `deposit`, `replace`, `balance_of` with the script disabled | the pilot table; decision |
| P2 | Second pilot: the call pair, `wp_call`, contracts as data, the slot-parametric `endLoan` | `bump_twice`, `take_and_bump`, `bump` | table |
| G1 | The scalar subset in full: every remaining combinator, loops as `withInvariant` | `Increment`, `Corpus`, `Loops`, `Rust`, `Aborts` | table |
| G2 | Calls in full, recursion, returned references | `Calls`, `Callees`, `References`, `Prophecies`, `Typed`, `Generics` | table |
| G3 | Storage in full: take, publish, contains, whole and focused brackets | `Storage`, `Account` | table |
| G4 | Delete `RowScript.lean`, the drive, and the per-layout laws; `verify` emits `leaner_normalize` only | whole corpus, scripts gone | final table; `Performance.exp` moved once with review |

Each milestone lands only with its gate met and its table in the
appendix. Ledger ports resume after G4, and from then on a port that
fails names a missing lemma, never a missing plan.

## Non-goals

- A reflective checker over a reified syntax tree. The combinators are
  the syntax; the normal form is a simp set over the goal.
- Changing the big-step semantics, the relational denotation, its
  agreement with the semantics, or the closer.
- Mutual recursion and generic recursive functions (their own item under
  *Recursion* in `certifying-execution.md`).

## Open questions

1. Whether the computation should be emitted by the generator next to the
   relation, or defined once as a function of the relation's combinators
   (`denoteRow : ExprDenotation → RowSpec` is not definable — the relation
   is not a program — so emission it is; the question is whether the
   agreement can be proved once per combinator with the generator only
   composing, as V1 does, or needs a per-body proof).
2. Loops with calls: `withInvariant` over a state whose registries change
   per iteration; whether the invariant quantifies the registries or the
   loop restores them.
3. How the two intended negatives report: the residue the closer cannot
   establish must be reported at the clause's range, as today.

## Appendix: baselines

To be recorded at P0: the scripts' heartbeats and objects for the pilot
targets (from `Performance.exp`) and for every check target.

## Composition implementation checkpoint (2026-09-04, in progress)

The opt-in `leaner.route "compose"` consumes a proved callee summary at
`wpFunction`, rather than unfolding the callee implementation. The current
scope is supported monomorphic, nonrecursive caller trees with exact abort
conditions. The normalizer retains the caller continuation at the call
boundary and reconciles only the newly exported pending suffix into the
caller frame. The inherited prefix is not replayed, including on a throw.

`Proofs/Composition.lean` strips typed argument/result transport in a shared
call rule, exposes the raw postcondition and frame, and composes the loan
discipline linearly across calls. Loan-free native aggregates use the
existing `Plain`/`LoanMatcher` algebra instead of searching their symbolic
payloads. Retired lexical markers have a row-independent identity law.

The added E2E `Composition` check covers value results, nested and repeated
calls, untouched locals and mutable parameters, independent mutable
parameters, and abort propagation. `ResourceComposition` ports the v0
two-family focused-write example. Both files have passed the unchanged
50M driver cap. The IR regressions check the exact frame after retiring a
local parent beside an untouched vector parameter, propagation of new loan
comparison facts, allocation-prefix discipline, and throwing-call suffix
reconciliation over arbitrary caller rows.

`Proofs/OperationWP.lean` connects the existing nominal focus algebra to
operation-level WP rules. Whole-referent and nominal-field reborrows use a
shared certified row update; cached resting-loan mutation validates both
the address and the loan stored there before updating the selected slot.
Unrecognized paths and stale caches retain semantic evaluation. The
sequencing normalizer must leave these operation boundaries available to
their rules, rather than consuming them first with generic `wp_evaluate`.
IR tests explicitly exercise the focused rules and the stale-cache fallback.

`Loans.splice` now passes the unchanged 50M driver cap from its actual
source path. The first connected-focus measurement was approximately
48.3M heartbeats and 46,816 proof objects (previously 52.2M and 51,698).
Loans is implemented pending a checkpoint, not yet a landed port.
Live-loan retirement still uses the runtime evaluator; general scoped
lifetime rules and broader call coverage remain unfinished. No existing
performance or failure baseline has been relaxed for this work.

`LeanerLang.Tests.CompositionPerformance` adds a fixed one/two/four-call
scaling gate beside an untouched mutable parameter. Its initial passing
measurements are 21.5M / 29.9M / 48.6M typed heartbeats and 21,679 / 29,443 /
44,182 proof objects. The new benchmark enforces the same 50M cap itself;
the existing `Performance.exp` remains unchanged and is still failing.

The next ports expose frontend boundaries too: `modifies R[k], *` now
retains a per-listed-family frame, and computed mutation targets receive
a typed local holder instead of triggering a global search for a consumed
temporary. The loose-frame and mathematical-specification checks now pass
the unchanged driver cap, including a two-family temporary-reference write,
an unchanged-key frame theorem, and both arithmetic boundary checks. They
are implemented pending checkpoint, not yet landed ports. Generic Boolean
callers consume a matching callee equivalence before connective splitting;
this preserves modular reuse across equivalent storage-key spellings.

The concrete generic-call bridge now recovers every scalar result shape
through shared canonical-decoder lemmas, including certified integers.
It does not assume that an arbitrary codec has a decode-to-encode inverse.
The callee is still proved once over its carrier and codecs; callers reuse
that theorem. Scalar-only callers build complete simplifier-lemma rows
before quotation, avoiding malformed empty-family splices. Their frame
proof inspects the known scalar slots instead of repeatedly case-splitting
runtime values and simplifying the entire context.

`GenericScalarCalls` passes ten capped checks, including strict specifications
with omitted abort clauses. `GenericStorage` also passes all seven checks.
The old `carry_u64` performance target now verifies, but costs 7.36M typed
heartbeats versus its 4.76M baseline. The existing performance gate remains
red on costs; no baseline has been relaxed. Fresh Move and Rust package
suites pass, and the IR suite fails only that old cost gate.

## Native-computation checkpoint (2026-09-05)

Priority correction: stop optimizing the retiring frame/row-script VC route.
`RuntimeFrame` remains part of execution and semantic agreement, but must not
occur in the computations or native contract proofs that replace that route.
The existing `typedDenotation` adapter alone does not meet this requirement:
it still decodes a runtime execution. Logical contract frame clauses remain
necessary and are distinct from runtime execution frames.

`Proofs/Computation.lean` introduces an exact `Represents` boundary between
an ordinary native `Spec` and execution on encoded arguments. It preserves
normal, failure, and undefined relations. Successful runtime results must be
canonical encodings; decoding alone is insufficient for an arbitrary codec.
The transport uses only the codec's left inverse. Pure native summaries can
retain exact state preservation without reopening the callee body.

`Proofs/ComputationAgreement.lean` contains the runtime-only agreement laws
for reads, moves at arbitrary local slots, operand sequencing, pure calls,
and function entry/export. Calls consume a callee agreement certificate;
the loan-free exit obligation is explicit, not inferred from a generic type.

The opt-in `leaner.route = "native"` consumes `computation`,
`computationVerified`, and `computationRepresents` artifacts. Missing
artifacts fail with row fallback disabled. It also rejects reuse of a theorem
already proved by another route. `#leaner_prepare` exposes the generated
execution denotation and authored contract without first verifying the
function through the old route.

`LeanerLang.Tests.NativeComputation` is an end-to-end **explicit-artifact
pilot**, not yet an automatic native-body generator. Its generic `carry` and
concrete `carry_u64` have native computations and modular native proofs, with
separate exact agreement and public source-semantics transport. A dependency
audit rejects execution-frame dependencies in their native proofs and also
rejects runtime-value encoding in their native body/argument/result artifacts.
The pilot still uses `RuntimeState` as the store type for compatibility with
the existing authored contracts; typed global-store computation is not yet
migrated. All proof stages are measured, including agreement and both transports:

| Pilot | Native aggregate heartbeats / objects | Original performance ceiling |
|---|---:|---:|
| `carry` | 1.44M / 2,168 | 1.96M / 2,568 |
| `carry_u64` | 3.40M / 4,530 | 4.94M / 8,126 |

The aggregate gate reads the unchanged `Performance.exp` ceilings. At this
pilot checkpoint, `Performance.lean` still used the retiring route; the pilot did not
claim to have migrated them. Next: generate the native computational bodies
and certificates compositionally, generate native contracts directly instead
of simplifying the old encoded contract adapters, and move the original
targets onto that path. Then extend it to effects, references, storage and
loops. Do not expand the old row scripts as the implementation strategy for
these steps.

Validation at this checkpoint: all three new test targets pass; fresh Move
and Rust package suites pass; `GenericScalarCalls` (10 checks) and
`GenericStorage` (7 checks) pass the unchanged 50M cap. The full IR suite
fails only `LeanerLang.Tests.Performance` on its existing cost regressions.

## Automatic native forwarding checkpoint (2026-09-05)

`LeanerLang/Computation.lean` now generates the native body, reusable native
summary, and exact execution certificate for one-parameter value forwarding:
reads/moves, generic-to-generic calls, and generic or non-generic scalar
callers. The original `Performance.lean` `carry` and `carry_u64` targets now
select `leaner.route = "native"`; they no longer run frame/row VCs or require
handwritten pilot artifacts. Unsupported bodies still fail closed.

The native summary retains the authored contract and adds an exact native
result equation plus state preservation. Caller VCs consume that summary
and establish the callee's precondition. A distinct purity certificate is
used only in the runtime agreement proof, never to bypass call preconditions
in the native contract proof. Generic callees remain carrier-parametric and
are not re-proved at each instantiation.

All generated proof artifacts, including the instantiated runtime theorem,
are charged to the existing typed target; public transport keeps its existing
target. `Performance.exp` is unchanged. Fresh full-suite measurements:

| Original target | Typed heartbeats / objects | Public transport heartbeats / objects |
|---|---:|---:|
| `carry` | 1,695,489 / 2,098 | 217,907 / 254 |
| `carry_u64` | 2,762,169 / 3,983 | 181,332 / 211 |

Both targets pass their original per-stage limits. Including both stages,
`carry_u64` uses 40% fewer heartbeats and 48% fewer proof objects than the
original baseline. The full IR suite still fails only the cost gate for
other targets on the retiring route; those baselines were not relaxed.

`NativeGenerated` tests six positive cases, rejects an unestablished call
precondition and false leaf/caller postconditions, and checks that failed
generation rolls back every native artifact. Its dependency audit excludes
runtime frames and execution/purity certificates from native proofs, and
runtime-value representation from native computations. The explicit pilot
and route-rejection tests also pass. Fresh Move and Rust package suites pass.

This is still a bounded pure fragment: no extra locals, effects, aggregates,
references, loops, or type-map-dependent contracts. Contracts still use the
existing encoded adapter and `RuntimeState`; direct native contracts and
typed global-store computations remain next work, alongside native operation
and sequencing rules. This checkpoint adds infrastructure coverage, not a new
v0 source-file port.

## Native checked arithmetic checkpoint (2026-09-05)

The native route now generates checked addition, subtraction and multiplication
over fixed-width integer parameters and integer literals. `NativeArithmetic`
checks the mathematical result and returns a `SpecInt` with its range
certificate or the exact failure payload. Its state and error types are
parameters. Generated source computations still instantiate the existing
`RuntimeState` and `Failure` compatibility types; failure-payload encoding is
not yet migrated to a fully native error vocabulary.

`ArithmeticAgreement` supplies shared exact operation-success, operation-failure,
and function-failure laws. These certify the generated computation against
execution without introducing execution frames into the native VC. Successful
exit still explicitly requires loan freedom; abort final states are existentially
hidden exactly as in the source semantics. No baseline or existing semantics
was weakened. Native generation uses the configured heartbeat cap for every
generated declaration, and failed arithmetic generation rolls back its artifacts.

The original `perf_calls::guarded` now uses this route, as does
`SpecLogicalArithmetic.checked_successor`. Fresh full-IR-suite costs for `guarded`:

| Stage | Heartbeats (baseline → current) | Objects (baseline → current) |
|---|---:|---:|
| Native generation, verification and agreement | 7,620,703 → 3,395,715 | 8,793 → 4,771 |
| Public transport | 188,858 → 189,242 | 211 → 211 |

This is a 54% aggregate heartbeat reduction and 45% smaller aggregate proof,
with unchanged `Performance.exp`. The earlier native `carry` and `carry_u64`
targets still pass. Other old-route cost regressions remain; the full IR
suite fails only the existing performance gate.

`NativeArithmetic` adds nine positive generated checks: arithmetic operators,
operand reversal/reuse, restricted preconditions, and literals on either side.
Three negative checks reject missing overflow conditions, spurious abort
conditions (sufficiency), and false postconditions. Rollback and dependency
audits pass. Seven IR boundary checks cover exact unsigned/signed range edges
and preservation of failure payloads with no runtime state/error vocabulary.
The source `SpecLogicalArithmetic` checks pass under the unchanged driver cap.

The next native work is expression/local sequencing and reusable summaries
for calls that may abort. This fragment does not yet support nested arithmetic,
extra locals, references, storage effects, or loops. Direct native contract
generation remains pending. No v0 file's port status is promoted here.

## Native aborting-call composition checkpoint (2026-09-05)

The native route now composes monomorphic integer callees that may abort.
It supports parameter-read operands, including reordering, and multiple
layers of callers. Native bodies apply the callee's typed computation;
native VCs consume its published `nativeSummary`, check its precondition,
and transport both its normal-return guarantees and declared aborts. They
do not unfold the callee body or consume execution/effect certificates.

`wp_of_satisfies_complete` retains conditional postconditions, must-abort
refutation on normal returns, and all allowed errors. The shared
`wp_of_stateFrame_satisfies` eliminates state-preserving call summaries
without assuming no abort at call entry. Arithmetic and generated callers
publish these strengthened summaries. A separate unconditional
`StatePreserving` certificate is used only by exact execution agreement.
`CallAgreement` handles normal results and existentially hidden abort states,
including the caller's operand evaluation and loan-free return boundary.

`NativeCalls` checks six positive callers, including a two-argument reversed
call and an omitted-abort contract. Five negative cases reject missing
preconditions, wrong results, missing/spurious aborts, and an abort property
true of the callee body but absent from its contract. Failed generation
must roll back all artifacts and the reusable registry entry. Dependency
audits exclude execution frames and semantic certificates from native VCs.
All generated helpers, summaries, effect/agreement certificates and public
transport are counted against unchanged original call-cost ceilings.

Fresh direct fixture measurements range from 2,956,757 to 4,616,668
heartbeats and 3,052 to 5,006 objects per caller, including every stage,
against the original `carry_u64` aggregate ceiling of 4,943,275 heartbeats
and 8,126 objects. The second caller layer costs 3,766,086 heartbeats and
3,489 objects; it reuses its callee summary rather than reopening the chain.

A core sequencing regression uses an opaque verified stage twice, including
an input that succeeds in the first stage and aborts in the second. This
tests the modular sequencing rule, not automatic source/local sequencing:
that generator work remains next. Extra locals, nested expressions,
references, storage effects, loops, and direct native contracts remain
outside this fragment. No v0 source-file port status changes here.

Fresh Move and Rust package suites and the capped `GenericScalarCalls`,
`GenericStorage`, and `SpecLogicalArithmetic` checks pass. The full IR suite
passes every target except the existing `Performance` cost gate on unmigrated
targets. Its native `guarded`, `carry`, and `carry_u64` targets still pass;
`guarded` now includes its reusable summary and effect certificate at
3,480,944 typed heartbeats and 4,892 objects, plus 189,263 heartbeats and
211 objects for transport. No performance or diagnostic baseline changed.

## Native typed-local sequencing checkpoint (2026-09-05)

The native route now generates straight-line variable bindings initialized
by checked integer addition/subtraction. Later steps can read those locals
and parameters, use integer literals, or return a saved value. Locals are
range-carrying `SpecInt` binders in an ordinary `Spec.bind` computation;
neither the computation nor its native VC uses a runtime frame or universal
value. Initializer and continuation aborts are both preserved.

`SequenceAgreement` factors execution from an already initialized frame
and proves shared checked-binding/continuation laws. Actual slot placement
and loan-free return conditions stay at this exact execution boundary. Native
verification follows one operation at a time, preserving the contract's
abort conditions until certified closing. State-preservation certificates
compose independently of the authored preconditions. A direct caller reuses
the sequenced callee's native summary, without reopening its implementation.

`NativeSequence` has ten positive checks, four rejected contracts, artifact
and registry rollback checks, and native dependency audits. Two independent
execution checks distinguish the exact `-1` initializer failure payload from
the `256` continuation failure payload. Full-IR-suite aggregate costs include
syntax generation, all generated proof artifacts, and public transport:

| Checked operations | Heartbeats | Proof objects |
|---|---:|---:|
| Two | 3,791,962 | 4,369 |
| Four | 6,923,883 | 6,367 |
| Eight | 17,303,412 | 10,622 |

The modular caller of the four-operation callee costs 3,606,677 heartbeats
and 3,473 objects. Gates use the unchanged original arithmetic/call budgets,
plus explicit chain-scaling checks: proof size stays within doubling when
the chain doubles; heartbeat growth is bounded below quadratic. This is not
a claim of linear elaboration time—the eight-operation search cost grows
more than twice the four-operation cost.

The next work is call-initialized locals and nested arithmetic operands.
Mutable-local assignments, heterogeneous locals, multiplication in sequences,
references, storage effects, and loops are not supported by this generator.
Direct native contracts also remain pending. This checkpoint adds native-route
infrastructure, not another completed v0 source-file port.

Fresh Move/Rust package suites and capped `GenericScalarCalls`,
`GenericStorage`, and `SpecLogicalArithmetic` source checks pass. The full
IR suite fails only the existing `Performance` cost gate on unmigrated
targets; the previously migrated native arithmetic/call targets still pass.
No cost or diagnostic baseline was changed.

## Native call-initialized locals checkpoint (2026-09-05)

Native straight-line bindings can now initialize a typed local from a
previously verified monomorphic integer callee. Call→arithmetic,
arithmetic→call, and repeated-call sequences all use ordinary `Spec.bind`.
The caller checks each callee precondition and consumes its `nativeSummary`;
it does not reopen the callee body, including when that body itself contains
a sequence. A checked-result WP rule names the native local and exposes its
value equation explicitly for subsequent modular calls.

`ScalarAgreement` supplies an exact scalar-expression boundary and a shared
binding/continuation law. Normal returns retain the local frame and use an
independently proved state-preservation certificate. Failure frames and
states remain existentially hidden; they are not assumed unchanged. Only
execution agreement uses these certificates and runtime slots. Native VCs
remain separate, as enforced by dependency audits.

`NativeLocalCalls` passes ten caller cases and six rejection cases, plus
artifact/registry rollback checks and two exact error-outcome checks. The
negative cases include facts true of the callee implementation but absent
from its contract: both an omitted abort condition and an overly precise
result equation must be rejected modularly. Positive cases also cover
reordered arguments, weak contracts, and a later precondition established
from an earlier result. Fresh full-IR-suite aggregate costs:

| Case | Heartbeats | Proof objects |
|---|---:|---:|
| Call then arithmetic | 4,819,529 | 5,534 |
| Arithmetic then call | 4,874,807 | 5,713 |
| Two calls | 5,991,762 | 6,429 |
| Four calls | 11,107,433 | 9,647 |

These include generation, native verification, semantic certificates, exact
agreement, and public transport. The unchanged original call/arithmetic
budgets pass, as does the check that doubling two calls to four costs no
more than twice the heartbeats or proof objects. Existing native-local
sequencing and migrated original performance targets still pass.

Move/Rust suites and the capped generic-scalar, generic-storage, and
logical-arithmetic source checks pass. The full IR suite still fails only
the older cost gate on unmigrated targets. No baselines or v0 port statuses
changed. Nested arithmetic/call operands, generic or heterogeneous call
bindings, mutable assignments, and effects beyond this scalar fragment
remain pending; direct native contract generation remains pending too.

## Native nested operands checkpoint (2026-09-05)

Checked addition/subtraction operands now compose recursively on the native
route, including both sides of an expression, local initializers and results,
and arithmetic sequences before/after modular calls. Native values are typed
integer products composed with `Spec.bind`; runtime operand rows occur only
in the exact agreement proof. `OperandAgreement` proves ordered operand
evaluation, checked-operation consumption, and scalar function finalization.
An operand failure prevents all later evaluation; normal-state preservation
is separate from the abort-state existentials.

`NativeOperands` passes eleven positive verifies, five rejected contracts,
rollback/dependency audits, and three exact error-payload checks. At input
zero, where both subtractions would fail, only the left failure is possible;
the right-only and outer-operation failures are tested independently.
Two/four nested additions cost 4.53M/8.04M aggregate heartbeats and
5,521/7,646 proof objects. The original unchanged arithmetic budgets and the
two-to-four doubling gate pass. Existing local-call costs are unchanged.
The full IR suite still fails only the inherited cost gate on unmigrated
targets; no performance baseline was changed.

The old `Generics` source fixture's generic-forwarding frame-entry regression
is repaired by selecting the native route, not by extending the old entry
rule. All four verifies pass the driver's unchanged cap. Its type assertions
now account for the explicit type map and do not depend on printed binder
names. The language ports for empty modules, abilities, and positional
structs also pass their capped checks; the test ledger records them separately.

Nested calls as operands, multiplication in sequences, heterogeneous/generic
local bindings, assignments, references/storage, and loops remain pending in
this generator. Direct native contracts and error vocabulary remain pending.

The subsequent enum source port passes five verifies and all nine original
interpreter assertions under the unchanged cap; positional structs retain
all four original interpreter assertions too. Boolean negation and length
now have checked operation descriptors (not yet native loop VCs). The
enum-payload and loop ports remain incomplete, with their actual capped
failures recorded in `test-organization.md` rather than accepted as passing
baselines. Native operand/local-call cost gates still pass after these changes.

Two issues must not be hidden by proof automation: v0 vectors carry a length
bound absent from today's native `Array` representation, and v0's signed
remainder contract omits the `MIN_INT % -1` abort present in both the actual
Move VM and current Leaner execution. Kernel boundary tests pin the latter;
the user approved matching the VM, and the new signed source port corrects
its remainder abort clause accordingly. All six verifies and twelve boundary
execution assertions pass under the unchanged cap. Execution semantics,
proof budgets, and the v0 reference tests were not changed to force parity.

## Integer-port and commit checkpoint (2026-09-05)

`Language/Integers` now passes all twelve verifies and 31 interpreter
assertions at the driver's actual 50M raw-heartbeat cap (`50000` in Lean
options). Checked casts have generated native computations and exact
execution-agreement proofs; the narrow/widen/signed cast regressions pass
at 5M raw heartbeats. Shared denotation descriptors additionally cover
bitwise operations, shifts, and logical operations with evaluator agreement.
Closed nested primitive constants are folded only with a recursively checked
agreement proof, including referenced initializers. Unsigned shift proofs
preserve masking and invalid-distance aborts; bitwise specifications use
mathematical integer operations, not runtime-value constructor enumeration.

The accumulated work is committed together with the qualifying ports listed
in `test-organization.md`. This is not an all-green milestone: the final
43-file Check audit has eight failures, including regressions in `Enums`
and `Loans`; neither is marked ported. `NativeLocalCalls.call_then_add` now
exceeds its unchanged aggregate heartbeat budget (5.108M versus 4.943M),
and the older `Performance` gate still fails. Move and Rust package tests
pass. Full E2E testing also exposes a printer fixed-point regression where
re-lowering a field assignment introduces a reference temporary. The latest
ledger audit supersedes earlier passing measurements above; no baseline or
budget was relaxed for this checkpoint.

## Native-call and enum-reference follow-up (2026-09-05, worktree)

The native local-call cost regression is fixed without changing any budget.
Closed arithmetic evaluator agreements and a shared integer-local
borrow-freedom theorem keep proof work at the boundary: `call_then_add`
costs 4.658M raw heartbeats against the unchanged 4.943M ceiling, and all
native local-call cost and scaling gates pass. The full IR test run now
fails only the older `Performance` root; no retiring frame route was
optimized to make that gate pass.

The next execution port, `Language/EnumRefs`, retains all sixteen functions
and ten assertions. It exposed a real discrepancy: mutable pattern payloads
were represented as data selections through mutable references, which the
execution-preparation validator correctly rejects. Lowering now introduces
one place reborrow per mutable payload binder and reuses that binding for
reads and writes. All ten execution assertions pass. Raw LIR's partial
field projections require explicit absent-variant guards in this port.

This is not a completed enum-reference checkpoint. Corrected lowering
regresses `Verification/EnumRefs`: `scale` and `overwrite_three` exceed the
50M cap. The other eight checks and all static path certificates now pass.
Static certificates enumerate the declaration's finite variants and use a
shared checked absence lemma for other names; they no longer simplify whole
namespace tables. A focused mutable-payload read/write/abort contract also
passes in the frontend regression suite under the unchanged cap.
The 44-file Check audit passes 35 files and fails nine; the ledger now
marks the verification row regressed rather than retaining its old ported
label. The printer's synthetic mutation-holder fixed point is repaired,
but reference-match canonicalization in `MoveStdlib/option` still blocks
the full E2E driver. No failure output was accepted as a passing baseline.

## Variant-aware operation checkpoint (2026-09-05)

The compositional borrow rule now handles enum fields as well as struct
fields. `FocusStep` retains the optional variant tag, and the generator
checks both owner and tag before constructing the same path certificate.
Its shared laws work with arbitrary payload siblings and surrounding locals;
this adds no per-function row shape or new execution assumption. Tests force
the focused rule alone to prove the operation for arbitrary tags and reject
a mismatched tag.

This brings `Verification/EnumRefs.overwrite_three` below the unchanged cap:
42.25M typed-proof heartbeats. Nine of the ten checks now pass. `scale`
remains over budget: its borrow steps use the compositional rule, but its
mutation and loan-retirement steps still use general evaluation. That is
the next missing compositional coverage, not permission to relax budgets
or add another function-layout-specific frame lemma.

The execution-only `Language/EnumRefs` port is complete (sixteen functions,
ten assertions, no v0 verifies). Its ported status is distinct from the
still-regressed verification file. The `option` printer fixed point is also
repaired: shared field aliases and stable mutable payload binders now print
in the form produced by lowering. A new seven-case Move source fixture pins
both reference kinds, value reads, writes, returned references, and shared
field names with uniform or heterogeneous payload types. The latter retain
typed matches, including when a concrete instantiation equates the types. The
latest audit in `test-organization.md` records the remaining suite failures.

## Enum-reference verification checkpoint (2026-09-05)

All ten `Verification/EnumRefs` checks now pass the unchanged Check driver
cap, including `scale`. The missing coverage was shared mutation evaluation
with a stale or absent loan-location cache. Its certificate proves cache
failure in stages and identifies the first matching local through the same
ordered search as execution. A direct resting borrower can then be updated
without reducing the whole evaluator. Earlier nested borrowers still take
precedence; negative tests ensure the focused rule declines those cases.
These are operation-level laws over arbitrary surrounding locals, not
function-layout-specific frame optimizations. Execution semantics are unchanged.

The evaluator's pre-simplification hook makes the certificate available after
operand evaluation as well as through the WP rule. Unit tests disable the
general mutation evaluator to check that registration and both missing/stale
cache paths independently. `overwrite_three` falls from 42.25M to 36.58M
typed-proof heartbeats. All ten actual driver checks pass; aggregate cost
measurements have a different scope from that per-command cap (`scale`
records 51.35M aggregate typed-proof heartbeats).

The latest 44-file Check audit passes 36 files, with eight failures recorded
in `test-organization.md`. Move and Rust package tests pass; only the older
`Performance` root fails in the IR package. Budgets and failure baselines
remain unchanged.

## Returned-reference composition checkpoint (2026-09-05)

All twelve checks in the existing `Verification/References` fixture now pass
the unchanged driver cap through `leaner.route "compose"`. Callee contracts
are consumed once; the caller does not re-verify an instantiated callee body.
The shared boundary handles mapped mutable results, sibling constraints from
resolved exports, global-loan registry summaries, and arbitrary ordered
pending-write suffixes. Cyclic summary equalities remain available as facts
but are not installed as expanding normalization rules.

Multi-loan retirement is staged by operation laws when resuming a call, with
inner-before-outer order preserved. No function layout is prescribed. Mutation
shares one validated registry lookup between its cached and searched rules;
scalar lender holes have a direct cache-miss certificate. The pair caller's
aggregate typed-proof cost drops from 56.42M heartbeats / 69,493 objects to
51.25M / 60,983. Aggregate accounting differs from the generated command's cap;
the actual driver-cap run passes independently.

The composition performance gate passes without baseline changes; Move and
Rust package tests pass. The older IR `Performance` root still fails. The
current Check audit and the remaining v0 ports are recorded in
`test-organization.md`; completing this hand fixture does not complete v0's
larger `ReturnedMutRefs` port.

## Literal-vector checkpoint (2026-09-05)

`Language/Literals` now passes both verifies and all seven execution assertions
under the unchanged driver cap. Its classifier costs 13.90M typed-proof
heartbeats / 8,516 objects. The shared finalizer uses typed loan-freedom to
certify that an aggregate's pruned borrow collection is empty without
traversing symbolic elements. A negative unit test retains an actual borrow.
The runtime comparison has a structural lawfulness proof, so vector branch
guards connect directly to specification equality. Logical closure is scoped
to vector guards: applying it to every context regressed `CrossInv`, and the
scoped version restores that check. Execution semantics are unchanged.

The composition performance gate still passes with its original baseline.
The next vector issue is the native representation's missing length bound:
v0's `Move.Vector` carries a `< 2^64` certificate, whereas the current native
array does not. `EnumPayloads.batch_length` needs that representation fact to
establish its unsigned result; weakening the contract is not the solution.

## Bounded-vector and enum-payload checkpoint (2026-09-05)

`Language/EnumPayloads` passes all eleven verifies and ten execution assertions
at the unchanged cap. Move-native vectors now carry v0's `< 2^64` length
certificate in arguments, results, locals, and nominal fields. The codec
checks element representations and length and has a proved roundtrip; Rust
and neutral array representations remain unchanged. The shared execution
semantics still use runtime vectors, and no VM operation was changed.

Returned enums compose by opening the generated decoder's own patterns,
consuming constructor and tag equations, and pruning impossible variants
before another call. Scalar callee summaries expose their literal payload
projections and closed variant tests before range reasoning. Callee bodies
are not re-verified. Injection is once-only for dependent equations, and
runtime-fact selection precedes any reduction of the fact's sides; unrelated
semantic equations remain folded. Both properties have 1M regression caps.

Generated enum loan-freedom uses the shared `Plain` law instead of opening
the borrow collector. The end-to-end check also rejects missing or admitted
vector/enum helpers and key verification theorems. `batch_length` costs 14.70M
typed-proof heartbeats / 10,856 objects; `choose_and_score` costs 38.23M / 37,396.

The full Check sweep is **39/44**, with no new failures. Move and Rust package
tests pass; only the older IR `Performance` root fails. The composition
performance baseline and all driver caps are unchanged. `test-organization.md`
records the five outstanding failures and the remaining v0 ports.

## Typed loop checkpoints (2026-09-06)

`Verification/LoopInvariants` and `Language/Loops` now use the generic
computational loop boundary. The checked native fixed point proves partial
correctness from entry, one invariant-preserving iteration, and exits;
iteration counts do not cause proof unrolling. The first port retains all
three verifies and six executions, including dynamically indexed vector
mutation. The language port retains fifteen functions, ten verifies, and
eighteen executions, including sequential/nested loops, labeled control, and
a resource borrow carried through a scalar countdown.

Authored clauses strengthen a default typed header invariant. Live locals
are decoded into their native carriers, iteration-local slots are optional
and loan-free, and other out-of-scope slots retain their entry values. The
last distinction preserves hidden resource owners after reference
initializers. Scalar loops retain their incoming loan-location cache through
a proved invariant; borrowing loops may grow the cache as the VM does. The
predicate choices are all subject to entry and preservation proofs, not
trusted effect assumptions. Invalid entry, preservation, exit, and default
invariants have rejection tests.

The latest capped Check audit is **50/50**; the separate performance gate
remains tracked in `test-organization.md`. The largest new language-loop
typed proof, `drain`, costs 30.18M heartbeats / 43,938 objects under the
unchanged 50M cap. No retired row-script recognizer, per-layout theorem,
execution-semantics change, or relaxed cost baseline was added for this port.

## Checked control expressions checkpoint (2026-09-06)

`Language/ControlForms` completes the next v0 port: 20 automatic,
admission-checked contracts and 43 runtime assertions, retaining all originals.
Executable short-circuit operators lower to branches; explicit core Boolean
primitives remain eager through print/reparse. Computed place indices are
sequenced once into typed temporaries, and closed Boolean/tuple matches use
coverage-checked decision trees. An entry guard preserves the exact zero-trip
state of ranges; their increments use the existing native assignment rule.

The proof changes are shared and bounded: substitute explicit scalar-literal
preconditions before normalization, split a scalar result's Boolean guard to
close tuple-pattern fallthrough, and use Boolean callee-summary equivalences
while normalizing continuation guards. The isolated call/match regression
drops from exceeding 50M heartbeats to 14.22M, without re-verifying the callee.
The supporting rules have 1M regressions. The capped Check corpus passes
51/51; separate integration/performance results remain in `test-organization.md`.

## Generic language checkpoint (2026-09-06)

`Language/Generics` completes all sixteen original automatic contracts and
39 runtime assertions (23 originals and sixteen type-family collision cases).
Generic structs, enums, and vectors retain native carriers in their contracts;
the same proved generic body is instantiated at a caller's invocation map,
not verified again. Arbitrary codecs are not assumed loan-free: the Move
boundary requires plainness of the actual encoded generic inputs.

Composition consumes encoded equality and returned-data summaries before
decoding the continuation. Constructor-oriented rewriting, bounded-vector
inversion, and scalar range-certificate consumption avoid searching the
execution context. The focused rules, including invalid-decoder rejection,
have 1M regressions; the driver and cost baselines are unchanged.

A reverse-call-graph worklist interns only the type instantiations demanded
by executable operations and transitive callees. This supplies body-only
resource keys without duplicating functions or enumerating unrelated type
combinations. Parameter-flow SCCs apply the VM's growing-instantiation-cycle
rule before closure; permutations and constant substitutions remain legal.
Later-namespace and nested-resource cases are tested. Logical inference also
refines from physical aggregate arguments, preserving print/reparse without
conflating distinct physical types.

The capped Check audit passes 52/52. The ledger records the remaining ports
and the six older cost-gate regressions; this is not performance parity.
