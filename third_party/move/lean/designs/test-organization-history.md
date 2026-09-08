# Test organization: historical plan and checkpoint notes

**Archive, not current status.** This preserves the previous ledger and its
checkpoint narrative through 2026-09-08. Use
[the current test ledger](test-organization.md) for pass/fail status and remaining
work. Claims such as "passes", "ported", "source checkpoint", "latest", and
"next" below refer to the checkpoint being described, not necessarily today's
native-only implementation. Some earlier passes used the now-disabled legacy
encoding. A historical pass or commit does not establish a current native pass.

The old term **source checkpoint** meant that a LeanerLang translation was
committed and passed at that time; it did not certify native encoding or absence
of later failures. Do not use it as a current completion label. This archive is
retained for provenance, original v0 counts, semantic decisions, and proof-cost
history; update current status in the main ledger instead.

## Generated scalar-owner mutation (2026-09-08, worktree)

`NativeMutable` now generates the first ownership-output source fragment:
literal integer assignment through one mutable fixed-width integer parameter,
with no extra locals or returned values. The native contract is translated
independently from authored clauses, not inferred from the assignment. Its
postcondition uses the typed updated value, `old` uses the entry value, and
conditional abort codes plus partial/strict/unspecified behavior retain the
source semantics. Native proof, exact execution agreement, finalization bridge,
and public source transport are separate generated artifacts. Generation is
synchronous at the existing cap and rolls back on any proof-stage error.

The registered `NativeMutableGenerated` suite has nine positive targets,
three false-contract rejections, five execution/result/loan assertions, and
admission/dependency guards. All stages cost 3.28–4.23M raw heartbeats / at most
5,384 objects, below the reference-test 10M/15k ceiling. The old route fixture's
unsupported literal writer is now a verified positive; a new reborrow example
preserves its unsupported-generation/rollback coverage.

The fresh full audit has the identical 19/61 pass list (39 native gaps, three
diagnostic mismatches). All registered IR/native roots except the same six
failing targets pass (200 jobs), as do Move (71 jobs) and Rust (165 jobs).
Prepared-call cost remains 9,923,105 heartbeats / 11,631 objects. No existing
budget or baseline changed. This is prerequisite source generation, not a new
completed Check-file port. Checked replacement expressions, additional locals,
reborrows, generic owners, and owner-returning callee summaries remain next;
`clear` and `drain` are still incomplete. Evidence:
`/tmp/dev3-native-owner-{unit-gate1,ir-gate1,move-gate1,rust-gate1,check-audit1}.log`,
`/tmp/dev3-native-owner-generator-build10.log`, and
`/tmp/dev3-native-owner-audit.qHGzPJ/`.

## Checked ownership-boundary pilot (2026-09-08, worktree)

`NativeBoundary` separates typed mutable-owner outputs from successful runtime
finalization. Its exact agreement and contract bridge retain the actual middle
state, abort/undefinedness behavior, and unconditional frame/must-abort duties.
The public verifier now accepts the separately checked monomorphic boundary
transport; native computation/VC dependency audits forbid the boundary adapter
and certificate themselves. The earlier `nativeOperation_update` identifier
error is fixed. Both boundary regressions are registered and passing.

The explicit `set_seven(&mut u64)` source pilot passes public verification,
native auditing, cached reuse, exact pending-write-back assertions, and
missing-write-back rejection. An ill-typed boundary certificate rejects with
no published verification artifacts. Its six proof stages cost 2,357,065 raw
heartbeats / 3,336 objects, below the reference-test 10M/15k ceiling. Generic
kernel regressions reject incorrect finalizers and postconditions and retain
frame/must-abort duties when a may-abort condition excuses the postcondition.
Kernel declarations use 1M caps with selected 10k-object proof guards.

This does not automatically generate mutable-body proofs or owner-returning
callee summaries. Reborrow lifetimes, generic boundaries, and source `clear`/
`drain` remain incomplete. No Check file is promoted: the fresh full audit is
identical at 19/61 (39 native gaps, three diagnostic mismatches). All registered
IR/native roots except the same six failing targets pass (198 jobs), as do
Move (71 jobs) and Rust (161 jobs). Existing performance caps and baselines are
unchanged; the tight prepared-call regression remains 9,923,107 heartbeats /
11,631 objects. Evidence: `/tmp/dev3-native-boundary-{unit-gate1,regressions1,
ir-gate1,move-gate1,rust-gate1,check-audit1}.log` and
`/tmp/dev3-native-boundary-audit.BfLfbX/`.

## Native mutation-kernel preparation (2026-09-08, worktree)

This batch is prerequisite work for `Verification/LoopInvariants.clear` and
`Language/Loops.drain`, **not a completed source port**. `NativeVector.replaceAt`
and checked `set` retain the typed element and bounded owner. Their WP reduces
to one bounds check, updated owner, and exact profile-selected failure; negative
indices do not become successful index-zero updates. Size, selected-element,
and unrelated-element laws support loop invariants without unfolding arrays.
`NativeMutation` retains `MutableArgument`'s native current value and loan
identity while updating the owner. Its agreement layer proves exact resolved
projection writes, checked failure/definedness, and the actual registered
reference-mutation evaluator at an arbitrary slot. No `RuntimeValue` or frame
is introduced into the native update computation.

The registered `LeanerIR.Tests.NativeVectorMutation` tests include symbolic
length/element/loan preservation, invalid bounds and wrong-output rejections,
exact vector and reference updates, and zero/three-iteration clearing. An
aggregate clearing loop proves every bit false and preserves its unrelated
field using one invariant step at an arbitrary vector length. Every test
declaration and the two new production mutation modules have a 1M-heartbeat
cap; selected kernel/agreement proofs have
10k-object guards and admission checks. This is kernel coverage, not a theorem
for the authored source `clear` or its borrow allocation/export behavior.
Enforcing the production cap exposed expensive commutative sorting of encoded
propositions in the write agreement; direct conjunction transport fixed it
without relaxing the new cap.

`ComputationAgreement.fromFrame_loop` no longer requires state-preserving
iterations: both normal and aborting continuations receive the actual
intermediate store. `NativeSequence` uses this generalized law and stops
generating that redundant agreement premise. New law-level regressions exhibit
a non-state-preserving iteration, its agreement, and a continuation observing
the changed allocation counter. Native regression/cost gates pass (the tight
prepared-call test remains 9.923M heartbeats / 11,631 objects); no existing cap
or baseline changed. The fresh Check audit remains 19/61 with the identical
pass/fail list (39 native-gap files, three diagnostic mismatches). Move (71
jobs) and Rust (160 jobs) pass; the same six IR targets fail. Source-level
borrow/reborrow handling and native loan-export integration remain next.

## Archived document

Status: plan, revised 2026-09-02 with the user's decisions (no Move
Prover differential; verification tests are baseline tests in the
end-to-end package; the v0 trees are ported one file at a time).
T0 and T3 are implemented; T1 is in progress. Historical source coverage is
18/23 verification, 19/20 language, and 7/9 negative files. These are source
checkpoints, **not completed native ports**. Native migration is incomplete;
the per-target native ledger below is the current completion evidence.
At the last committed checkpoint, all 60 Check files
passed the complete audit at the unchanged caps; active worktree results and
native migration are recorded separately below. The separate IR performance gate
still has previously recorded regressions. See the latest checkpoint below.
T2 is not started.

**Native-only cutover (2026-09-07, worktree):** fallback is off now, not after
the remaining ports. The default is `native`; all retired route selections
reject, including attempts to reuse cached proofs. Every `verify` audits native
computation/VC dependencies automatically. `RowScript.lean` and the source
normalization/composition emitters have been deleted. All acceptance route
selectors now request native verification. Unsupported bodies fail: the suite
is deliberately not reported as green, and merely changing a route selector
does not promote a test. The cutover census is **17/61 passing, 44 failing**:
42 files expose native implementation/proof gaps; two differ only in expected
diagnostics. Six IR test targets fail; the Move and Rust suites pass. Counts
refer to affected files, not independent bugs or completed verification ports.
The **60/61** mixed-route vector audit completed before this cutover and is
historical only.

**Latest native-only audit (early-return step, 2026-09-08): 19/61 passing, 42 failing.**
`Language/Integers` now passes all twelve verified targets and its interpreter
assertions, including the final shared-reference case. Seven additional indexed
read targets, six state-free `Verification/Aborts` targets, and
`ControlForms.compound_local`, `clamp`, `then_else`, and `checked_assert` now pass
inside still-incomplete fixtures. `Verification/Loops` now passes completely;
ten v0 `Language/Loops` targets (plus two return-scope regressions) and two
`Verification/LoopInvariants` targets pass
explicit native audits. Thirty-nine files still expose native implementation/
proof gaps; three differ only in diagnostics, including the now-native rejected
loop-invariant contracts.
The same six IR targets fail; all native regression/cost gates pass. Move
(71 jobs) and Rust (160 jobs) pass. No fallback was re-enabled.

## The two needs

1. **A benchmark plugged into the ground.** v0 (the deprecated `move`
   package) verified its acceptance corpus with mostly bare `verify`
   commands, and its language and negative tests pinned elaboration,
   compilation, and diagnostics.  The leaner stack must reach parity on
   that corpus and then be measured against it:
   - [`move/Move/Tests/Verification/`](../move/Move/Tests/Verification/):
     23 files, 174 `verify` commands, 99 bare (automatic) and 75 with
     proof scripts; `Quicksort.lean` (3 verifies, ~220 proof lines) and
     `OrderedMap.lean` (9 verifies, ~480 proof lines) carry the
     AI-generated proofs.
   - [`move/Move/Tests/Language/`](../move/Move/Tests/Language/): 20
     files covering the language surface (integers of every width, signed
     arithmetic, enums and their payloads and references, generics,
     loops, tuples, vectors, addresses, attributes, abilities).
   - [`move/Move/Tests/Negative/`](../move/Move/Tests/Negative/): 9 files
     pinning diagnostics (borrow errors, returned `&mut` shapes, lowering
     and surface rejections, spec-function and specification errors,
     verification failures).
2. **Every end-to-end test verifies.** We map one language with
   specifications into LeanerLang, and the produced LeanerLang must
   verify.  Today no end-to-end test elaborates its output.

## What exists today

| Suite | What it checks | Verification |
|---|---|---|
| `leaner-ir` `LeanerIR/Tests` (16 files) | IR validation, semantics, interpreter, codecs | none (unit tests) |
| `leaner-ir` `LeanerLang/Tests` | elaborator (`Frontend`), three hand row proofs, 10 `Verification*.lean` fixtures (46 verifies), the `Performance` gate (20 targets) | yes, on hand-authored LeanerLang |
| `leaner-move` `Tests` (2 files) | profile, intrinsics | none |
| `leaner-rust` `LeanerRust/Tests` | driver, registry, source backend, 9 LeanerLang programs | none |
| `leaner-e2e-tests` `MoveToLeanerLang` | 14 direct `.move` files plus MoveStdlib (14 modules, 106 spec blocks), AptosStdlib (6), AptosFramework (2): printed LeanerLang compared textually and re-parsed for a canonical fixed point | none |
| `leaner-e2e-tests` `RustToLeanerLang` | 53 `.rs` fixtures, same textual baseline | none |
| `leaner-e2e-tests` `MonoVM`, `MonoDifferential` | linked MonoVM smoke test; three-engine differential execution | none (execution) |

The gap: the corpus that defines "done" is not run by anything, the
end-to-end output is text that nobody elaborates, and the verification
fixtures that do exist are a small hand-picked set with no relation to
either.

## The convention

A **check** is a LeanerLang source file under
`leaner-e2e-tests/LeanerE2ETests/Check/`: a `.lean` file that imports
`LeanerLang` and contains `leaner module` declarations with
specifications, `verify` commands (bare, or with an inline proof script
where real mathematics lives, as v0 did), and whatever other commands the
test needs.  **Everything under `Check/` is LeanerLang** — no Move
sources, no Rust sources, and not the v0 surface: a v0 test is ported by
rewriting it in LeanerLang.  The frontend paths keep their own
directories (`MoveToLeanerLang`, `RustToLeanerLang`).  The driver
elaborates the file and records its diagnostics.

- **The output is the baseline, verbatim**, side by side as `<name>.exp`,
  the way compiler-v2's baseline tests record theirs
  (`move-prover/test-utils/src/baseline_test.rs`,
  `verify_or_update_baseline`): the `.exp` is exactly what `lean` prints
  for the file — `LeanerE2ETests/Check/Verification/WrongIncrement.lean:12:2:
  error: verification failed for …` and the message body as the user
  reads it — with no reformatting.  The only cleaning is compiler-v2's:
  trailing whitespace trimmed, a single trailing newline.  The driver
  invokes `lean` on the package-relative path, so the path in the
  message is stable across machines without rewriting.
- **A clean check has no `.exp`.**  As in compiler-v2, `UB=1` writes the
  file when there is output and **removes** it when there is none, and
  a missing file is an empty expectation.  Positive and negative tests
  are therefore the same kind of file; a negative test is one whose
  `.exp` exists — a verification failure at its clause range, a rejected
  borrow, an unsupported construct.  Every severity is recorded, so an
  unexpected warning is as visible as an error; tests set options locally
  when they mean to silence something.
- **Each file runs in its own `lean` process** with the package's search
  path, so a runaway proof or a crash in one check cannot take the driver
  down, and under a per-command heartbeat cap that the driver passes
  (start at 50M, five times the largest generated route), so a proof that
  silently falls back to search surfaces as a diff instead of a slow
  suite.  Cost stays gated precisely by `LeanerLang/Tests/Performance`;
  the cap only catches collapses.
- **Discovery, not registration.** Every `.lean` under `Check/` is a
  check; subdirectories mirror the v0 categories.

```text
leaner-e2e-tests/LeanerE2ETests/
  Check/
    Verification/
      Account.lean            # clean: no .exp
      Quicksort.lean          # proofs inline, clean: no .exp
      WrongIncrement.lean     # negative
      WrongIncrement.exp      # LeanerE2ETests/Check/Verification/WrongIncrement.lean:12:2: error: verification failed …
    Negative/
      Borrows.lean
      Borrows.exp
    Language/
      Integers.lean
  MoveToLeanerLang/           # unchanged (see T2 for the verifying stage)
  RustToLeanerLang/           # unchanged
  MonoVM/, MonoDifferential/  # unchanged
```

`LEANER_E2E_SUITE=check` selects the suite; `lake test` runs it with the
others.  `LeanerIR.TestInfra.Baseline` gains compiler-v2's semantics
(`check` today always writes the file, even an empty one): write when
the output is non-empty, remove when it is empty, trim trailing
whitespace.

## Historical source-coverage ledger

The v0 trees are ported one file at a time — each file rewritten in
LeanerLang, since the v0 surface is a different language — each check
landing only when it passes: a clean run for a positive test, the
intended diagnostics for a negative one.  The deprecated file stays as
the reference until its port lands.  The ledger below is kept current in this document; a row
moves to **source checkpoint** with the commit that lands it.

**Source checkpoint** records that the source translation was committed and its
check passed the driver's unchanged cap at that checkpoint. Earlier versions
called this "ported" even for legacy-backed verification; that label did not
establish the required native migration. The source files and checkpoint history
are unchanged. Subsequent regressions are marked explicitly.

**Native complete** requires every verification target in the file to select
native without fallback and pass the native computation/VC dependency audit,
plus its functional, negative, agreement, and unchanged performance gates.
Neither a commit nor a passing legacy-backed test satisfies this condition.
Completed native fixtures also end with `#leaner_require_native_all`: this
checks every successful source verification declared in that file, including
targets missing an individual assertion. Imported proofs are excluded. A
regression test deliberately verifies a function by the old route and checks
that both the individual and whole-file audits reject it.
Until the compatible state/contract adapters are replaced, even these native
VC checks do not establish fully native storage and contract representation.
**Checkpointed draft** means the source is committed for continuity but does
not qualify as a passing source checkpoint. The current checkpoint contains both kinds of file;
commit status is no longer an outstanding step for these rows.

| v0 file | verifies (proofs) | status |
|---|---|---|
| `Verification/AbortDirections.lean` | 5 | **source checkpoint** (2026-09-02, `Check/Verification/Aborts.lean`; all five verify, `withdraw` through the path-generic bracket and the absent-resource abort direction) |
| `Verification/Account.lean` | 2 | **source checkpoint** (2026-09-02, `Check/Verification/Account.lean`; its `#test` execution assertions are left to the differential suite) |
| `Verification/BorrowCertificates.lean` | 0 (certificates only) | **source checkpoint** (2026-09-04, `Check/Verification/BorrowCertificates.lean`; the identity reference exports one shared parameter certificate with no loans or diagnostics) |
| `Verification/Callees.lean` | 13 (3 proofs) | **source checkpoint** (2026-09-02, `Check/Verification/Callees.lean`; `bump`, `bump_twice`, `take_and_bump`, the added `add_two`/`bump_then_add_two` (a callee whose exit value is not the entry plus one), the unspecified-helper callers `calls_pure_helper`, `embedded_helper`, `helper_condition`, `set_pair`, `forward_set_pair`, and `bump_counter` verify; `drain` and `call_drain` verify since 2026-09-03 through the recursive fixed point, where v0 needed hand proofs over the recursive contract) |
| `Verification/Calls.lean` | 10 (3 proofs) | **source checkpoint** (2026-09-02, `Check/Verification/Calls.lean`; `twice`, `increment`, `increment_unspecified`, `add_to`, `effect_caller`, `read_counter`, `choose`, `call_choose` verify, and `recursive_choose` since 2026-09-03 through the recursive fixed point, where v0 needed a hand proof over the recursive contract) |
| `Verification/CorePrimitives.lean` | 5 | **source checkpoint** (2026-09-04, `Check/Verification/CorePrimitives.lean`; explicit global/local borrow-read-write, explicit abort, checked vector indexing, and local indexed assignment all verify automatically under the 50M-heartbeat cap) |
| `Verification/CrossInv.lean` | 1 | **source checkpoint** (2026-09-04, `Check/Verification/CrossInv.lean`; quantified cross-resource module invariants are assumed on entry and proved on exit, specialized to explicit modified keys; the two-family erase/insert chain verifies automatically at 23.95M typed heartbeats) |
| `Verification/EnumRefs.lean` | 10 | **source checkpoint** (2026-09-05; all ten checks, including `scale`, pass the unchanged driver cap with stable mutable payload reborrows. Shared mutation certificates cover missing and stale loan-location caches without changing execution semantics. `overwrite_three` costs 36.58M typed-proof heartbeats, down from 42.25M at the execution checkpoint.) |
| `Verification/GenericStorage.lean` | 7 | **source checkpoint** (2026-09-04, `Check/Verification/GenericStorage.lean`; generic storage bodies verify once over typed carrier/codecs and concrete callers specialize those theorems through a sparse invocation type map) |
| `Verification/GlobalBorrows.lean` | 6 | **source checkpoint** (2026-09-06; all six original checks and admission guards pass the unchanged 50M cap through normalization. Indexed-loan search/update certificates avoid traversing symbolic vectors, and bounded pure vector reads use a scoped arithmetic inventory.) |
| `Verification/GlobalInv.lean` | 5 | **source checkpoint** (2026-09-04, `Check/Verification/GlobalInv.lean`; ordinary and update module invariants, implicit existence guards for total resource reads, and publish/remove/multi-family mutation obligations all verify automatically under the 50M-heartbeat cap) |
| `Verification/Invariants.lean` | 5 (3 proofs) | **source checkpoint** (2026-09-04, `Check/Verification/Invariants.lean`; struct, enum, and resource data invariants are assumed for typed inputs and re-established for results, mutable-reference exits, and modified resources; copyable by-value enum matches use a v0-style variant decision tree, and all five verify automatically under the 50M-heartbeat cap) |
| `Verification/Loans.lean` | 3 | **source checkpoint** (2026-09-06; all three original checks and admission guards pass the unchanged 50M cap. `extend` and `independent_element` use normalization; `splice` composes the proved `extend` contract twice. A syntactically guarded product-range rule avoids the unrelated u64 vector-length recursion blow-up. As in v0, `clear` is specified but not verified.) |
| `Verification/LoopInvariants.lean` | 3 | **source checkpoint** (2026-09-07 revalidation, `Check/Verification/LoopInvariants.lean`; all three original verifies, admission guards, and six zero/multi-iteration execution checks pass under the unchanged 50M cap. Negative entry, preservation, and weak-exit invariants reject with specific errors. Shared optional-slot, indexed/shared-borrow, vector decoding/re-encoding, and immutable-snapshot rules have 1M regressions. Revalidated in the vector checkpoint's complete 55/55 audit.) |
| `Verification/LooseFrame.lean` | 2 | **source checkpoint** (2026-09-05, `Check/Verification/LooseFrame.lean`; both original checks and an added temporary-reference/two-family write verify under the unchanged driver cap. A kernel-checked theorem also confirms that other keys in the listed family remain unchanged. No `.exp`.) |
| `Verification/OrderedMap.lean` | 9 (9 proofs) | not ported |
| `Verification/Quicksort.lean` | 3 (3 proofs) | not ported |
| `Verification/Read.lean` | 2 (2 proofs) | **source checkpoint** (`Check/Verification/Read.lean`; both contracts verify automatically under the driver cap; no `.exp`) |
| `Verification/ResourceComposition.lean` | 1 | **source checkpoint** (`Check/Verification/ResourceComposition.lean`; sequential focused writes to distinct resource families verify under the driver cap; no `.exp`) |
| `Verification/ReturnedMutRefs.lean` | 66 (51 proofs) | not ported |
| `Verification/SpecFunctions.lean` | 12 | not ported |
| `Verification/SpecLogicalArithmetic.lean` | 1 (1 proofs) | **source checkpoint** (2026-09-05, `Check/Verification/SpecLogicalArithmetic.lean`; the original successor check, unverified scan, and two added mathematical-arithmetic boundary checks all elaborate under the unchanged driver cap. Boolean decoding preserves the determined witness; vacuous equivalence directions close by constructor contradiction. No `.exp`.) |
| `Verification/Summaries.lean` | 3 | not ported |
| `Negative/Verification.lean` | 2 expected failures | **source checkpoint** (2026-09-06; both original incorrect contracts fail at their source clauses under the unchanged cap, and neither exports a `verified` theorem) |
| `Negative/BorrowGlobals.lean` | 1 expected rejection | **source checkpoint** (2026-09-06; execution preparation rejects removal of a resource with a live shared loan, with the exact conflict and related-origin diagnostic. Three positive cases pin ended loans, distinct families, and non-invalidating existence checks. Structural unit materialization alone does not check executable borrow safety.) |
| `Negative/Surface.lean` | 6 expected rejections | **source checkpoint** (2026-09-07; all six original rejection categories, two added continue-operand checks, five Move/Rust execution cases, and canonical round-tripping pass in the complete 56/56 audit) |
| `Negative/Specifications.lean` | 3 cases | **source checkpoint** (2026-09-07; non-resource existence/modifies clauses reject with positioned ability diagnostics. The former `old(address)` rejection is now an admission-free positive proof, with two execution checks and canonical round-tripping. No implementation changes.) |
| `Negative/Borrows.lean` | 5 original proofs, 1 rejection | **source checkpoint** (2026-09-07; all originals plus an owner-restoration proof, seven execution cases, two final-state assertions, Rust rejection, admission guards, and canonical round-trips pass. The positioned rejection checks fit a 1M guard. Move direct-local mutable borrows with an unused let result retire before the body; used aliases and projected/global/Rust borrows retain their conflict checks.) |
| `Negative/IntrinsicUnsupported.lean` | 2 original cases | **source checkpoint** (2026-09-07; intrinsic attributes are now implemented. The incomplete owner fails full Move-schema validation with four missing-role diagnostics; the orphan `map_spec_get` role fails positioned source lowering. A complete four-role graph validates and canonically round-trips through the full Move schema. This is schema coverage, not native-map execution support.) |
| `Negative/Lowering.lean` | 2 original proofs, compiler diagnostics | **source checkpoint** (2026-09-07; six source/LIR rejection checks, five proofs with admission guards, thirteen executions, and canonical round-trips pass. The former Lean-host vector helpers use the shared-primitive spellings, as in VectorOperations; this does not claim executable imported standard-library bodies. Body metadata cannot replace authored semantics, recursive Move types reject, host/unresolved calls have positioned diagnostics, and additional contract resource families and mutual calls are covered positively.) |
| Remaining `Negative/` (2 files: `ReturnedMutRefs`, `SpecFunctions`) | | not ported |
| `Language/EmptyModule.lean` | 0 | **source checkpoint** (2026-09-05; empty module elaboration and semantic preparation pass the unchanged driver cap) |
| `Language/Abilities.lean` | 0 | **source checkpoint** (2026-09-05; all eight declarations, generic ability bounds, and phantom profile metadata pass the unchanged driver cap) |
| `Language/Attributes.lean` | 0 | **source checkpoint** (2026-09-06; all original declaration-metadata assertions, specification-function annotations, and `move_public` compatibility pass the unchanged cap. Nested calls and assigned values survive print/reparse; mixed intrinsic/user metadata and four malformed intrinsic annotations have additional regressions.) |
| `Language/PositionalStructs.lean` | 2 | **source checkpoint** (2026-09-05; all four functions, both original verifies, and four original interpreter assertions pass the unchanged driver cap; positional syntax maps to named `_0`/`_1` fields and explicit constructor patterns) |
| `Language/Enums.lean` | 5 (1 proof) | **source checkpoint** (2026-09-06; all five functions, five original verifies, and nine original interpreter assertions pass the unchanged cap. Delaying Boolean witness enumeration in call-free proofs reduces `mixed_match` from 51.68M to 37.41M typed-proof heartbeats. All five verification theorems are checked for admissions.) |
| `Language/EnumPayloads.lean` | 11 | **source checkpoint** (2026-09-05; all 13 functions, 11 verifies, and ten original execution cases pass the unchanged cap. Move-native vectors carry their u64 length bound, including enum fields; Rust keeps its native arrays. Returned-enum decoding and variant pruning compose through callee summaries. `batch_length` costs 14.70M typed-proof heartbeats and `choose_and_score` 38.23M. Generated vector/enum helpers and key verification theorems are checked for admissions.) |
| `Language/Signed.lean` | 6 | **source checkpoint** (2026-09-05; user resolved the semantic discrepancy: match the VM. All ten functions, six verifies, and twelve VM-boundary interpreter assertions pass under the unchanged cap. The port adds the `MIN_INT % -1` remainder abort omitted by v0 and checks its exact overflow payload; neither v0 nor VM semantics changed.) |
| `Language/Arithmetic.lean` | 12 | **source checkpoint** (2026-09-05; all twelve functions and verifies, all twenty-nine original scalar execution cases, and all nine original resource-store cases pass under the unchanged cap. Checked unsigned quotient/remainder facts are exposed without case enumeration; structural integer inequality normalizes directly. Resource cases pin both the resulting store and cleaned loan state, including pre-borrow arithmetic aborts.) |
| `Language/Tuples.lean` | 10 | **source checkpoint** (2026-09-05; all ten functions and verifies and all ten original interpreter assertions pass under the unchanged cap. Tuple locals and parameters use heterogeneous native products, with the VM's flat `RuntimeValue.tuple` introduced only by the boundary codec. Composed tuple-returning calls invert that codec structurally without enumerating runtime-value constructors.) |
| `Language/Addresses.lean` | 2 (+1 helper) | **source checkpoint** (2026-09-05; both original verifies, a variable-key read helper, and six equivalent interpreter assertions pass under the unchanged cap. Source-package aliases are represented by their resolved 256-bit literal at the LeanerLang boundary. Runtime address equality closes as logical equality without unfolding String's byte implementation; fixed-address resource reads compose through the typed family-presence bridge.) |
| `Language/EnumPatterns.lean` | 6 | **source checkpoint** (2026-09-06; all six original verifies and eight interpreter assertions (five originals plus three direct cases) pass the unchanged 50M cap. Branch-sensitive normalization prunes unselected continuations, and direct branch/enum agreement dispatch avoids relational tactic backtracking. All six public verification theorems are checked for admissions.) |
| `Language/Literals.lean` | 2 | **source checkpoint** (2026-09-05; all five functions, both verifies, and all seven interpreter assertions pass the unchanged cap. Typed loan-freedom discharges the finalizer's aggregate collector, and lawful runtime equality connects vector guards to specification equality. The classifier costs 13.90M typed-proof heartbeats. Typed vector literals preserve LeanerLang's intentional distinction from `Bytes`; the abort message remains a byte literal.) |
| `Language/Integers.lean` | 12 | **source checkpoint** (2026-09-05; all twelve verifies and 31 interpreter assertions pass the unchanged driver cap. Checked narrowing/widening casts, unsigned bitwise masks and shifts, exact cast/shift abort payloads, nested computed constants, decision-tree matches, conditional effects, and shared primitive references are covered. Native cast regressions additionally pass a 5M raw-heartbeat cap.) |
| `Language/EnumRefs.lean` | 0 | **source checkpoint** (2026-09-05; this execution-only v0 file has no verifies. All sixteen original functions and ten execution assertions pass the unchanged driver cap. Mutable match payloads use one stable reborrow per arm. Explicit variant guards represent v0's checked field projections because raw LIR field selection is partial.) |
| `Language/Vectors.lean` | 6 | **source checkpoint** (2026-09-06; all six original contracts and executions pass the unchanged 50M cap, with admission guards. Shared insertion/removal primitives preserve bounds and removal transfers both the removed value and remaining vector. The `make` helper has an additional proved no-abort clause for modular callers. Exact bounded-vector result decoding and loan-discipline composition after a call each have 1M regressions.) |
| `Language/Loops.lean` | 10 (6 proofs) | **source checkpoint** (2026-09-06; all fifteen functions, ten original contracts, and eighteen executions pass the unchanged 50M cap, with admission guards. Typed default invariants handle unannotated loops; the second phase retains the bound from v0's manual proof. Labeled exits/continues use existing IR nesting depths. Hidden resource-borrow owners survive scalar loops, with proved cache preservation. Structured loop/no-helper/early-return assertions and source round trips replace the corresponding v0 CFG checks.) |
| `Language/ControlForms.lean` | 18 (6 proofs) | **source checkpoint** (2026-09-06; all 22 original functions, 18 original verifies, and 37 original executions are retained. Two additional Boolean functions/contracts and six executions cover short-circuit OR, eager core operations, and computed-index overflow. All 20 verifies are automatic under the unchanged 50M cap, with admission guards and canonical print/reparse.) |
| `Language/Generics.lean` | 16 | **source checkpoint** (2026-09-06; all 25 original functions, 16 automatic verifies, and 23 original executions pass the unchanged cap, plus 16 resource-family collision executions. Admission guards, resource-access/binder/call-instantiation checks, and canonical print/reparse pass. Generic bodies remain shared over native carriers; demanded resource types propagate through actual calls without cloning bodies.) |
| `Language/VectorOperations.lean` | 25 | **source checkpoint** (2026-09-07, `Check/Language/VectorOperations.lean`; all 33 original functions, 25 verification requests with admission guards, 36 execution checks, and canonical round-trip checks pass under the unchanged 50M cap. Native vector errors match the VM rather than v0's blanket library abort code.) |
| Remaining `Language/` (1 file: `BorrowChecker`) | | not ported |

Totals to reach: 174 verifies (99 automatic, 75 with proofs) across the
23 verification files.  v0 files also carry `#test` execution assertions
over the lowered module; a port keeps them where the leaner surface has
an equivalent and otherwise leaves execution to the MonoVM differential
suite, which owns it.

### Completed lowering-diagnostics checkpoint (2026-09-07)

The complete capped Check audit passes **60/60**: **18/23 verification,
19/20 language, 7/9 negative** source ports. `Negative/Lowering` passes five
proofs, thirteen executions, source/LIR rejection checks, admission guards,
and canonical round-tripping. Recursive Move nominal definitions are rejected
at shared validation, including cycles through vectors, generic arguments,
variant fields, function signatures, and dependency interfaces. Rust retains
its recursive nominal types. The checker visits the interned type/declaration
graph rather than expanding paths: 512-level shared diamond graphs pass the
1M guards, and all 512 directed graphs on three declarations match an
independent bounded path-search oracle. Unknown source calls now carry spans.

Symbolic vector bounds expose guards before their continuations normalize.
The closer resumes only residual computations and requires progress; data-only
obligations avoid another normalization pass. A generic vector-projection
equation exposes the bounded read without expanding its payload. The added
two-read case exercises repeated guard introduction, with both successful and
aborting executions. No contract or budget was weakened. IR functional and
composition-performance gates and Move/Rust suites pass. The same six legacy
performance regressions remain; no baseline was changed.

**Priority update (2026-09-07):** complete migration of verification to native
encoding before further source ports or retiring-route performance tuning.
The port counts below describe source coverage, not native-route coverage.
At that pre-cutover checkpoint, `normalize` and `compose` used encoded rows;
both are now disabled. The following records describe the native migration
steps before the cutover. Shared native control-flow support was the next
migration step. No additional source file is promoted by this priority change.

Native migration is tracked independently of those source-port counts:

| Acceptance file | Newly native targets (worktree) | Remaining route coverage |
|---|---|---|
| `Language/Arithmetic` | All ten scalar targets, including division and remainder | Resource-reference `multiply` and `divide` still need native implementation. |
| `Language/Signed` | All five scalar arithmetic targets | Resource-reference `credit` still needs native implementation. |
| `Language/ControlForms` | Fifteen targets: `branch_effect`, `match_effect`, `echo_flag`, `if_let_action`, `embedded`, `arithmetic_condition`, `explicit_arithmetic_condition`, `short_circuit_and`, `short_circuit_or`, `eager_core_and`, `index_arithmetic`, `compound_local`, `clamp`, `then_else`, `checked_assert` | Other statement control flow, loops, mutable references, and tuple matching remain. |
| `Language/Integers` | All twelve verified targets, including `primitive_match_reference` | Complete fixture, native audits, and interpreter assertions pass in the native-only audit. |
| `Language/Literals` | `fixed_address` | Vector classification still needs native implementation. |
| `Language/Vectors` | `make`, `length`, `middle` | The indexed caller reuses `make`'s native summary. `replace`, `insert_middle`, and `remove_middle` still fail closed; this is not a native-complete fixture. |
| `Language/VectorOperations` | `borrowed_length`, `nested`, `bool_round_trip`, `read_out_of_bounds` | Shared reads/length and exact bounds failure pass the native-artifact audits. Mutation and the other remaining operations are not native-complete. |
| `VectorBounds` | `read_bad`, `read_reference` | Literal bounds failure and shared-parameter indexing pass native audits. Mutable indexing and the other remaining targets still fail closed. |
| `Language/Enums` | All five verified targets, including `total` and `mixed_match` | Complete fixture and interpreter assertions pass in the fresh audit. |
| `Language/EnumPatterns` | `nested_total` and its five constructor-argument callers | Complete fixture and interpreter assertions pass in the fresh audit. |
| `Language/PositionalStructs` | `first`, `destructure_move_spelling` | Both verified targets and interpreter assertions pass natively in the fresh audit. |
| `Verification/Generics` | No new targets; native-artifact assertions added to all four existing native proofs | The carrier example now uses `computation`, not the encoded `typedDenotation` adapter. |
| `Verification/GenericScalarCalls` | All ten verified targets | Native-artifact checks and eight generic theorem-reuse guards pass. |
| `Verification/Aborts` | Six state-free targets: `halve`, `bump_checked`, `identity`, `successor`, `explicit_abort`, `assert_floor` | Resource-based `withdraw` remains a native gap. The intentional wrong-postcondition case still fails. The whole fixture is not promoted. |
| `Verification/Loops` | `count_to`, `count_to_with_continue` | Complete fixture passes with explicit native audits. |
| `Language/Loops` | `count_down`, `count_down_loop`, `two_phases`, `shadowed_loop_state`, `shadowed_loop_arrow` | Nested/labeled control, early returns, and resource-reference state remain native gaps. |
| `Verification/LoopInvariants` | `count_to`, `sum_ones` | Authored invariants over multiple mutable scalar locals pass; mutable aggregate/vector `clear` remains a native gap. |

Each listed migration selects `native` explicitly and uses
`#leaner_require_native`, which rejects a legacy typed theorem and audits the
native computation/VC artifacts. This gate does not claim that the compatible
`RuntimeState`/failure vocabulary has already become a fully typed global
store. Shared conditional rules cover typed scalar comparisons, Boolean
results, nested branches, and Boolean/integer locals. Native aggregate support
adds monomorphic struct/enum construction, nested constructors, aggregate locals
and results, variant predicates, and payload-free classification. Nine new
positive aggregate checks cost 0.84–3.18M raw heartbeats per target across all
generated stages, with false-contract rejection and four exact native execution
assertions; no budgets or baselines changed. Runtime frames occur
only in the separate agreement proofs. Native branch regressions include
false contracts, abort preservation, and aggregate proof-cost checks. The
remaining native coverage includes effectful conditions, assignments,
generic aggregate operations, reference/storage effects, recursion, and loops.

The payload step adds nested native enum matches with exact selected-path
projection certificates, irrefutable nested struct destructuring, and modular
owned calls with native constructor/projection arguments. Callee bodies stay out
of caller VCs. Ten positive native payload/destructuring checks now pass cleanly,
including guarded arithmetic and payload preconditions, at 3.31–43.19M raw
heartbeats across all generated stages. Four modular callers cost 4.06–4.81M.
Two false contracts are rejected without leaking native artifacts, and four
exact native execution assertions pass. Literal array projection normalization
is shared; selected-path premises are normalized without revisiting the entire
WP. A transient Lean panic diagnostic while handling payload range certificates
was eliminated by introducing their arithmetic bounds directly; the passing
targeted run has no panic diagnostics. The complete payload-step Check audit
passes **60/61**, with only the existing `Language/Generics.choose` timeout.
Move (70 jobs), Rust (121 jobs), and IR functional/native/composition-cost tests
pass; the legacy Performance gate still has seven regressions. This is native
migration, not additional v0 source ports.

The next owned-call/local-binding step now passes eight positive native tests,
two negative contracts (one forbidding use of an unpublished result), explicit
summary-reuse/native-artifact checks, and three exact execution assertions.
All generated stages cost 1.00–13.01M raw heartbeats. Two aggregate calls cost
8.25M and four cost 13.0M, passing a 2× scaling guard; older native local-call
and payload cost checks remain green. The shared call emitter replaces the
integer-only local-call proof generator and supports aggregate results in typed
locals and subsequent matches/calls. Its fresh integration audit passes
**60/61**, with only `Language/Generics.choose` over budget. Move (70 jobs) and
Rust (122 jobs) pass; IR reports only the same seven legacy Performance
regressions.

The Boolean pattern/equality and Boolean-call step now passes nine positive
native tests, one false-contract rejection without artifact leaks, summary-reuse
checks, and four exact execution assertions. All generated stages cost
1.34–5.06M raw heartbeats, and the previous native cost guards remain green.
`ControlForms.match_effect`, `echo_flag`, and `if_let_action` select explicit
native and require native artifacts; the complete file passes. Equality uses
typed operand representations rather than the descriptor's Boolean result type.
Move (70 jobs) and Rust (122 jobs) pass, and IR has only the same seven legacy
Performance regressions. The new full Check audit passes **60/61**, with only
`Language/Generics.choose` over budget. The next native work is
effectful operands in comparisons/constructors and short-circuit conditions.
No caps, baselines, source-port counts, or checkpoint status changed.

The effectful-constructor step now passes seven positive tests, two false
contracts (including incorrect operand-failure order), summary-reuse/artifact
audits, and four exact execution assertions. Typed products compose fields in
source order, including nested constructors and mixed Boolean/integer fields;
their scalar agreement preserves operand aborts. Typed local aggregate results
feed subsequent projections and modular calls. All generated stages cost
1.31–6.16M raw heartbeats, and every exact execution assertion passes a separate
1M cap. `ControlForms.embedded` selects explicit native, and the full fixture
passes. The full Check audit remains **60/61**, with only `Language/Generics.choose`
over budget; Move (70 jobs), Rust (125 jobs), and IR functional/native cost checks
pass. The seven legacy Performance regressions remain unchanged. No source-port
promotion, budget change, or checkpoint commit was made. The next condition
composition edits are not included in this completed constructor audit.

The condition-composition step passes nine positive native tests, two false
contracts with rollback checks, and seven exact execution assertions. Native
conditions now include arithmetic operands, local Boolean results, nested
short-circuiting, and conditional aborts. All generated stages cost 4.69–7.35M
raw heartbeats; exact execution assertions each pass a separate 1M cap.
`ControlForms.arithmetic_condition`, `explicit_arithmetic_condition`,
`short_circuit_and`, and `short_circuit_or` select explicit native with artifact
audits, and the complete fixture passes. Move (70 jobs), Rust (127 jobs), and
IR functional/native cost checks pass; only the same seven legacy Performance
regressions remain. The full Check audit passes **60/61**, with only
`Language/Generics.choose` over budget. No budgets, baselines, port counts, or
checkpoint status changed.

The eager-Boolean step passes nine positive native proofs, two rejected
contracts with rollback checks, and five exact execution assertions. Both
operands execute in order; eager `and`/`or` preserve failures even when their
Boolean answer is already determined. All generated stages cost 1.65–7.77M
raw heartbeats, and exact assertions each pass a separate 1M cap.
`ControlForms.eager_core_and` selects explicit native with an artifact audit;
the full fixture and its interpreter assertions pass. The complete Check audit
is **60/61**, with only `Language/Generics.choose` over budget. Move (70 jobs),
Rust (127 jobs), and IR functional/native cost checks pass; only the same seven
legacy Performance regressions remain. No caps, baselines, source-port counts,
or checkpoint status changed.

The division/remainder step passes thirteen positive native proofs, two rejected
contracts with artifact rollback, a modular-summary reuse guard, and seven exact
execution assertions. Typed result computations preserve zero-divisor payloads,
truncation signs, quotient overflow before remainder, and ordered operand
failures. Shared arithmetic evaluation facts no longer live in the legacy WP
module. All generated stages cost 2.41–7.03M raw heartbeats; exact execution
assertions each pass a separate 1M cap. Four scalar targets in `Arithmetic` and
`Signed` now require native artifacts; both full fixtures and interpreter checks
pass. Move (71 jobs), Rust (132 jobs), and IR functional/native cost checks pass;
the same seven legacy Performance regressions remain. The full Check audit
passes **60/61**, with only `Language/Generics.choose` over budget. No caps,
baselines, source-port counts, or checkpoint status changed.

The scalar-constant step passes eight positive native proofs, two rejected
contracts with artifact rollback, and six exact execution assertions. Integer,
address, string, and byte-array constants use the shared pure-value generator;
the separate agreement checks their actual runtime literals. Byte-array literals
are also supported in authored specifications. All generated stages cost
0.84–2.30M raw heartbeats; exact execution assertions each pass a separate 1M
cap. Eight `Integers` targets, `Literals.fixed_address`, and all ten
`Verification/GenericScalarCalls` targets now explicitly select native with
artifact checks. Eight caller dependency guards enforce generic theorem reuse.
The full targeted fixtures pass; IR functional/native cost checks, Move
(71 jobs), and Rust (132 jobs) pass. The same seven legacy Performance
regressions remain. The full Check audit passes **60/61**, with only
`Language/Generics.choose` over budget. No budgets,
baselines, source-port counts, or checkpoint status changed.

The bitwise/consolidation step passes thirteen positive native proofs, two
rejected contracts with rollback, a modular-summary reuse check, and seven exact
execution assertions. AND/OR/XOR preserve fixed-width signed interpretation and
operand-failure order. Nested masks, conditions, constructors, locals, and a
callee continuation use the shared native path. Arithmetic, constructors, and
conditions now share `NativeOperands` for their typed operand products, WP,
state-preservation, and agreement generation. All generated bitwise stages cost
2.23–5.65M raw heartbeats; execution assertions each pass a separate 1M cap.
`Integers.masked` requires native artifacts, and the complete fixture passes.
IR functional/native/composition cost checks, Move (71 jobs), and Rust
(135 jobs) pass; the seven legacy Performance regressions remain. The most
recent full Check audit is still the preceding constant/scalar-call **60/61**
audit, not a full audit of this step. Shifts follow below. No budgets, baselines,
source-port counts, or checkpoint status changed.

The native shift step passes twelve positive proofs, two rejected contracts
with full native-artifact rollback checks, and seven exact executions. Shift
counts retain their own integer widths; wrapping, signed extension, invalid
distance payloads, nested operand failures, locals, and modular continuations
agree with execution. Two dependency guards confirm that shift callers reuse
the callee's native summary. `Integers.shifted` and `Integers.halved` now select
native explicitly; only `primitive_match_reference` remains legacy in that file.

Profiling found expensive kernel checking of a native VC, not a slow agreement
proof. A direct right-shift range theorem replaces the redundant division
equation in generated arithmetic contexts. The local shift/add proof drops
from 46.57M to 3.36M heartbeats; shift/add/call drops from 30.63M to 5.55M.
All twelve targets cost 2.51–5.55M raw heartbeats and 3,667–7,585 objects over
all generated stages. The new suite enforces tighter 10M-heartbeat and
15,000-object aggregate gates, plus 1M per exact execution assertion. Existing
baselines and budgets are unchanged. IR passes apart from the same seven
legacy Performance regressions; Move (71 jobs) and Rust (137 jobs) pass.
The fresh full Check audit is **60/61**, with only the already-known
`Language/Generics.choose` budget failure. No source-port promotion or checkpoint
commit is implied. Vector construction/length migration follows below.

The native vector step passes seventeen positive proofs, two false-contract
rejections with artifact rollback, six callee-summary reuse guards, and seven
exact executions under separate 1M caps. Native values include bounded vectors,
nested vectors, vector parameters/locals, and nominal vector fields. Constructors
sequence arithmetic and modular-call operands with the shared native operand
combinator, preserving first-abort order. Length reads the native array's size
and its existing certificate, without decoding elements. Scalar calls in derived
specification bodies now receive the same explicit logical-to-physical injection
as locals when stored in a vector; executable call types remain unchanged and
mismatches still reject. Frontend tests check both boundaries.

All seventeen targets cost 1.13–6.97M raw heartbeats and 1,812–8,298 objects over
all generated stages, under new 10M/15,000 aggregate guards. The registered
`NativeVectors` suite enforces the whole-file native audit. `Language/Vectors`
selects native explicitly for `make` and `length`; its remaining four reference
consumers are not native ports. IR functional/native cost checks, Move (71 jobs),
and Rust (140 jobs) pass; IR retains the same seven legacy Performance regressions.
The full mixed-route Check audit passed **60/61**, with the known
`Language/Generics.choose` timeout. This result predates the native-only cutover
above and is not the current suite result. No existing budgets, baselines, or
source-port counts changed.

The shared-reference step adds fifteen native positive proofs covering scalar,
Boolean, vector, and monomorphic struct reads; checked/nested arithmetic;
conditions; local copies and shared local borrows; projections; and shared
callee arguments. Source copy and immutable-local-borrow operations have exact
agreement laws; neither adds runtime decoding or a loan allocation to the native
computation. Generic `Spec.bind` associativity lets the copy law retain operand
effects without extra native binds. Boolean reference contract binders now use
their logical referent domain. Exact abort payloads normalize at arithmetic
proof leaves, without unfolding callee bodies.

`NativeReferences` checks two false-contract rejections with complete artifact
rollback, two callee-summary reuse guards, seven exact computation/execution
assertions, and a whole-file native audit. All fifteen targets cost 1.04–5.88M
raw heartbeats and 1,732–5,700 objects over all generated stages, under new
10M/15,000 aggregate guards; exact assertions have separate 1M caps.
`Language/VectorOperations.borrowed_length` now passes an explicit native audit.
The fixture as a whole is still failing and is not promoted. The final
native-only Check audit improves to **18/61**, with `Language/Integers` now
complete. The same six IR targets fail; native regression/cost gates, Move
(71 jobs), and Rust (141 jobs) pass. Mutable references, reborrows, indexing,
storage, and loops remain incomplete. No performance baselines, existing caps,
or source-port counts changed.

### Native checked-indexing checkpoint (2026-09-07, worktree)

`NativeIndex` passes thirteen positive proofs with native-artifact audits,
three false-contract rejections with complete artifact rollback, two explicit
callee-summary reuse guards, and eight exact result/error assertions. Coverage
includes literal/dynamic/shared indices, nested vectors, arithmetic indices,
`u8` elements with `u64` indices, and indexing a vector produced by a verified
callee. All generated stages cost 2.04–7.21M raw heartbeats and 2,371–7,520
objects under unchanged 10M/15,000 guards; exact assertions retain a 1M cap.

The native operation checks bounds and selects the typed array element.
Separate agreement laws preserve the source check/borrow sequence and abrupt
control. Index arithmetic overflow precedes bounds failure, whose Move error
remains `runtime.vector_error` with payload `1`. Caller length premises and
indexed observations normalize at the specification boundary; callee bodies
and runtime vector decoders remain folded.

Seven targets in four existing fixtures now have explicit passing native
audits: `Vectors.middle`, `VectorOperations.nested`, `bool_round_trip`,
`read_out_of_bounds`, `ControlForms.index_arithmetic`, and
`VectorBounds.read_bad`/`read_reference`. The fixtures themselves still fail,
so none is promoted as a completed native port. The complete native-only Check
audit remains **18/61**, with 41 native-gap files and two diagnostic mismatches.
The same six IR targets fail; native regression/cost gates, Move (71 jobs), and
Rust (142 jobs) pass. No fallback, existing budget, or baseline was changed.

### Native local-assignment checkpoint (2026-09-08, worktree)

Straight-line assignments use typed native rebindings, with separate exact
block/assignment laws preserving slot bounds and abrupt control. Pure local
aliases propagate directly, and new bindings invalidate only the affected
variant facts. Explicit local layouts keep the execution-agreement proof from
repeatedly reducing nested updates: the swap regression improves from exceeding
50M heartbeats to 3.24M. No runtime frame enters a native computation or VC.

Checked multiplication now composes with effectful operands and assignments;
unsigned products receive nonnegative certificates from their typed operands.
Abort payloads keep their computed VM values. Named Boolean result equations
close logical equivalences before splitting, while constant negations normalize
before contradictory equations can loop in the simplifier.

`NativeAssignments` passes twelve positive proofs, two rejected contracts with
full artifact rollback, a callee-summary reuse guard, six exact result/error
assertions, and whole-file native auditing. Coverage includes repeated writes,
saved values, swapping, Boolean/nominal/vector locals, calls, and symbolic/nested
multiplication. All generated stages cost 1.78–6.03M raw heartbeats and
2,172–6,752 objects, under unchanged 10M/15,000 guards. Exact assertions and the
Boolean-closer regression retain separate 1M caps.

`Language/ControlForms.compound_local` now passes an explicit native audit; the
fixture still fails and is not promoted. The complete Check audit remains
**18/61** (41 native-gap files, two diagnostic mismatches). The same six IR
targets fail; dedicated native-computation regression/cost gates, Move (71 jobs),
and Rust (143 jobs) pass. Statement-position branches and unit-valued effects
remain prerequisites for native loop consumers. No fallback, existing budget,
baseline, or source-port count changed.

### Native conditional-assignment checkpoint (2026-09-08, worktree)

Branches assigning the same local now join through a typed value, including
nested choices, checked arithmetic, calls, structs, and vectors. General
execution-agreement laws preserve the condition's intermediate state, abrupt
control, and assignment bounds. They normalize only the statement head, not
its continuation. No runtime frame enters the native computation or VC.

Pure integer choices become a single typed conditional value. Projection
normalization exposes its arithmetic value; effectful branch results retain
their value equations and range facts. Calls nested in a branch are reported
to preceding statements, so those statements still use the callee-compatible
proof form. Pure scalar definitions do not introduce unrelated value aliases.

`NativeConditionalAssignments` passes eleven positive proofs, two rejected
contracts with complete artifact rollback, a callee-summary reuse guard, five
exact result/error assertions, and whole-file native auditing. Two structural
guards check a single arithmetic continuation and a single join around nested
pure choices. Aggregate costs are 2.58–9.84M raw heartbeats and 3,000–11,631
objects under the unchanged 10M/15,000 guards; exact assertions have 1M caps.
The repeated-branch case improves from 12.04M to 9.24M without raising its cap.

`ControlForms.clamp` and `then_else` have explicit native artifact audits;
the fixture remains incomplete and is not promoted. The fresh complete audit
remains **18/61**, with 41 native-gap files and two diagnostic-only mismatches.
No previously passing file regressed. The same six IR targets
fail; all dedicated native regression/cost gates, Move (71 jobs), and Rust
(143 jobs) pass. Multi-local joins, unit-valued effects, early loop control,
mutable references, and typed storage remain unfinished. No legacy fallback,
existing baseline, or source-port count changed.

### Native Unit/abort checkpoint (2026-09-08, worktree)

Native computations now support Unit results and locals, discarded call
results, nested Unit blocks, statement-position assertions, and explicit
aborts with computed u64 codes. General agreement laws preserve operand
evaluation order and intermediate-state failures. Normal results use typed
values; runtime failure payloads remain an exact execution boundary.
Unit callees reuse their verified summaries. A targeted normalization of the
callee's no-abort fact eliminates encoded argument equalities without reopening
its body. Embedded `spec` obligations are deliberately not erased: unsupported
assertions fail closed until a native obligation translation consumes them.

The registered `NativeStatements` suite passes fifteen positive proofs, four
rejection/rollback guards (including `spec assert false`), four callee-summary
reuse guards, eight exact result/error assertions, and whole-file native
auditing. Exact tests distinguish first-failure ordering, division's empty
abort payload, and arithmetic overflow's computed payload. All generated
stages cost 0.75–6.37M raw heartbeats and 1,470–7,462 objects under 10M/15,000
guards. The tight existing conditional-call case remains below 10M (9.923M).

`ControlForms.checked_assert` and the six state-free `Verification/Aborts`
targets pass explicit native audits. Both fixtures remain incomplete. The
fresh complete Check audit remains **18/61**, with exactly the previous pass
list, 41 native-gap files, and two diagnostic-only mismatches. The same six IR
targets fail; dedicated native regression/cost gates, Move (71 jobs), and Rust
(144 jobs) pass. No fallback, existing baseline, budget, or source-port count
changed. Typed loop translation/invariants, multi-local joins, Unit arguments,
mutable references, and storage remain unfinished.

### Native loop kernel checkpoint (2026-09-08, worktree; not a source port)

`NativeLoop.run` carries an ordinary typed local state and typed exit value
through `Spec.withInvariant`. Its checked finite-execution characterizations
preserve success, aborts, and undefined behavior, including intermediate store
updates. Divergence produces no invented timeout result. The WP rule requires
invariant entry and one iteration proof; it does not unroll a chosen count.

`NativeLoopAgreement` connects this computation in both directions to the
structured execution loop. Normal iterations and unlabeled continues repeat;
an unlabeled valueless break exits. Other control is explicitly excluded by
the iteration certificate, not discarded. The owned-local continuation law
passes one typed exit state into one continuation. Runtime frames occur only
in these execution-agreement statements, not the loop computation or WP.

The registered `LeanerIR.Tests.NativeLoop` passes under a 1M-heartbeat cap per
declaration: arbitrary countdowns, a store-changing invariant, exact later
abort payloads, undefined behavior, divergence, a concrete local update and
break, continue agreement, and rejection of outer-loop control. A false
invariant cannot remove real executions or prove a false postcondition.
Thirteen explicit axiom audits reject admissions; representation guards reject
runtime frames/values in the native loop declarations. The full IR gate retains
the same six failing targets. Source loop translation, typed authored-invariant
binding, and labeled/return control remain unfinished: this checkpoint promotes
no acceptance file and does not change the 18/61 native audit.

### Native source-loop checkpoint (2026-09-08, worktree)

The source generator now consumes the typed loop kernel. Header state is a
heterogeneous integer/Boolean product, and authored invariants project its
typed values directly. The unused frame/existential invariant builder and
its decoding helper/test have been retired; native invariant-representation
guards cover the replacement. Inline non-loop `spec` obligations still fail
closed rather than disappearing as runtime no-ops.

Typed statement joins preserve normal, continue, and break paths with one
continuation. The loop VC checks entry and one iteration; modular calls reuse
native summaries. Separate exact agreement laws retain execution frames only
at the boundary. The registered `NativeLoops` suite passes twelve positive
proofs, four negative/rollback cases, five exact generated-computation tests,
native/admission audits, callee-reuse checks, and fixed-point/branch-duplication
guards. The exact tests cover immediate break, zero and multiple iterations,
and selected/unselected aborts under 1M-heartbeat caps. Aggregate proof costs
include invariant generation: 0.987–10.305M raw heartbeats and 2,139–10,199
objects under new 20M/30,000 guards. Existing caps and baselines are unchanged.

The complete native-only Check audit improves to **19/61**. Its pass list
changes only by `Verification/Loops`. Three `Language/Loops` targets and two
`Verification/LoopInvariants` targets have explicit native audits; neither
whole fixture is promoted. Remaining failures are 39 native-gap files and
three diagnostic mismatches (`Negative/LoopInvariants` now reaches the intended
invalid obligations). The six known IR failures remain; all native regression/
cost gates, Move (71 jobs), and Rust (150 jobs) pass. The next loop gaps are
body-local lifetimes, nested/labeled control and returns, reference/aggregate/
generic headers, and store effects. Source-port counts do not change.

### Native loop-local checkpoint (2026-09-08, worktree)

Body-local lifetimes are now native for owned scalar temporaries. Normal joins
carry the locals available at that point; break/continue carry only the loop
header. The execution agreement separately relates live values to a frame,
using typed optional dead cells only as proof witnesses. Both execution
directions are proved, with exact intermediate stores and errors. Completeness
produces a reachable frame; it does not pretend that every observed frame is
reachable. A shared linear proof checks the absence of loans in dead cells.
Neither frames nor optional dead cells enter generated computations or VCs.

The registered `NativeLoopLocals` suite passes twelve positive proofs, two
negative cases with full artifact/invariant-metadata rollback, four exact
generated-computation executions, four explicit law-admission guards, one
callee-summary reuse guard, and structural dead-cell/no-duplication checks.
It covers shadowing, loop temporaries, normal branch joins, branch-local
temporaries, continue, early break, Boolean/Unit locals, abort payloads,
sequential loops, an authored invariant, and modular calls. Pure integer
aliases retain explicit value equations for later invariant obligations.
All generated stages cost 1.260–11.529M raw heartbeats and 2,763–12,761 objects
under new 20M/30,000 guards; exact tests retain 1M caps. The existing tight
conditional-call regression remains at 9.923M under its unchanged 10M cap.

`Language/Loops.shadowed_loop_state` and `shadowed_loop_arrow` now have explicit
native audits, bringing that incomplete fixture to five audited targets.
The fresh complete Check audit remains **19/61** with the identical pass list,
39 native-gap files, and three diagnostic mismatches. The six known IR failures
remain; native regression/cost gates, Move (71 jobs), and Rust (154 jobs) pass.
No fallback, baseline, existing budget, or source-port count changed. Next:
nested/labeled control and returns, then the remaining reference/aggregate/
generic loop state and store effects.

### Native nested-loop checkpoint (2026-09-08, worktree)

Nested and labeled loop control now uses a heterogeneous sum of typed enclosing
headers. Each loop consumes its own break/continue and propagates the selected
outer header; lexical temporaries are not stored in the outer loop state.
Execution agreement proves exact control-depth unwinding, successes, failures,
and absence of undefined executions. The control encoding is agreement-only.
The compact single-loop computation/emitter remains unchanged.

The registered `NativeNestedLoops` suite passes nine positive proofs, two
negative cases with full artifact/invariant-metadata rollback, four exact
generated-computation executions under 1M caps, four law-admission checks, and
structural checks against encoded/dead cells, duplicated loops, and duplicated
arithmetic. Cases cover labeled exits/continues, three nesting levels,
heterogeneous headers, normal exits with lexical temporaries, nested aborts,
and an authored inner-loop invariant. Aggregate proof costs are
2.409–17.231M raw heartbeats and 4,678–13,630 objects under new 20M/30,000
guards. Existing performance gates are unchanged; the tight conditional-call
case remains at 9.923M under its 10M cap.

`Language/Loops.labeled_exit`, `labeled_continue`, and `labeled_proof` now pass
explicit native audits, bringing that still-incomplete fixture to eight
audited targets. Its remaining verification errors are the storage-borrowing
`drain` and the early-return `early`; no whole-file promotion is claimed.
The fresh complete Check audit remains **19/61**, with the identical pass list,
39 native-gap files, and three diagnostic mismatches.
The same six IR targets fail; native regression/cost gates, Move (71 jobs),
and Rust (158 jobs) pass. Returns still need source generation and transport,
even though the nested agreement law already preserves returned control.
Reference/aggregate/generic loop state and storage effects remain unfinished.
No fallback, existing budget, baseline, or historical source-port count changed.

### Native early-return checkpoint (2026-09-08, worktree)

Statement-position early returns, including returns through nested loops,
now carry a native typed result at the bottom of the loop-target sum. Normal
paths alone execute the shared continuation. `fromFrame_returning_discard`
proves exact execution at the function boundary, including return-operand
failures and loan export; the return encoding and frame observation are
agreement-only. Non-returning paths keep their existing emitters.

The registered `NativeEarlyReturns` suite passes eleven positive proofs, two
negative cases with complete artifact/invariant-metadata rollback, nine exact
native executions under 1M caps, three law-admission guards, one callee-summary
reuse guard, and structural no-frame/dead-cell/duplicated-continuation checks.
Coverage includes integer/Boolean/Unit returns, checked operands and overflow,
returns from locals and nested loops, normal continuation joins, bypassed
aborts, and modular calls. All generated stages cost 2.297–11.475M raw
heartbeats and 4,351–12,365 objects under new 20M/30,000 guards. Pure return
values feed the postcondition directly, without an opaque alias. Existing
budgets remain unchanged; the tight conditional call stays at 9.923M.

Source lowering no longer strips a return merely because it ends a nested
block. Only a true function-tail return can become a fallthrough value.
Unit returns produce zero result values while retaining any operand effects.
`Language/Loops` adds native audits for `early` and `return_in_loop` (ten v0
targets audited); its sole remaining verification gap is the storage-borrowing
`drain`. Two additional native scope regressions execute the source IR:
returning a computed local from a loop, and an effectful Unit-return operand
whose abort must not be discarded (three interpreter cases).

The fresh complete Check audit remains **19/61**, with the identical pass list,
39 native-gap files, and three diagnostic mismatches.
The same six IR targets fail; all native regression/cost gates, Move (71 jobs),
and Rust (160 jobs) pass. This does not claim support for every value-position
abrupt expression, returned reference, or effectful/generic loop header.
Those and typed storage effects remain migration work. No fallback, baseline,
existing budget, or historical source-port count changed.

The next `Negative/ReturnedMutRefs` probe is **not ported**. Checking the actual
VM corrected the initial diagnosis: `reference_safety::AbstractState::ret`
rejects both local and global roots still borrowed at return. Direct and bound
global-reference returns must therefore be negative cases, as in v0, not new
positive coverage. The active work adds that shared validation rejection and
corrects the hand `Verification/References` fixture to borrow storage in the
caller and return a parameter-derived reference from its helper. Separately,
shared finalization is being made aware of returned handle identities so a
resting local alias is not retired prematurely. Valid parameter reborrows,
aggregate aliases, and unrelated dying loans exercise that change.
The new negative source fixture passes its two parameter-return proofs,
executions, all five original rejection categories (plus direct/shared global
returns), and round-tripping. It is not checkpointed or counted as ported yet:
the full `References` fixture now passes, including the replacement
storage-backed caller and its final state. Prefix-registration, retirement,
alias-free boundary, and key-normalization laws pass the 1M checks. The latest
complete integration audit after native payload migration passes **60/61**:
`Language/Generics.choose` still exceeds the unchanged cap on its unmigrated route.
`Verification/Callees` now passes after guarding finalization normalization.
The native-migration work briefly exposed a separate `Language/Enums`
logical-closing regression; narrowing the new Boolean rule fixed it, and the
fresh full audit confirms the fix. The generic cap failure remains an active
regression, not accepted diagnostics. Move/Rust suites and IR functional tests pass;
composition performance passes after removing needless equation traversal.
The legacy performance gate has the six previous regressions plus
`forward_reborrow` (about 12% above baseline). Validation continues without
changing baselines or acceptance caps.

### Completed borrow/intrinsic-diagnostics checkpoint (2026-09-07)

The complete capped Check audit passes **59/59**. Source-port counts are
**18/23 verification, 19/20 language, 6/9 negative**. All five original
`Negative/Borrows` proofs and an additional owner-restoration proof pass with
admission guards. Seven execution cases and two final-state assertions check
that unused competing handles retire without leaking loans or losing writes.
The source borrow analysis permits only an unobserved direct-local Move
mutable borrow in this rule; the result must be unused by the entire let body.
It does not propagate non-use into arbitrary initializer computations, nor
relax projected/global borrows, live aliases, or Rust. An empty active-loan
set and non-candidate lets skip the additional liveness walk. Both rejection
checks fit a 1M guard; all verification remains at the existing 50M cap.
No runtime or representation change was needed.

`Negative/IntrinsicUnsupported` replaces obsolete blanket rejection with
current source/schema diagnostics and validates a complete role graph before
and after canonical reimport. IR functional and composition-performance gates
and Move/Rust suites pass. The six recorded legacy performance regressions
remain; no baseline or budget was changed.

### Completed specification-diagnostics checkpoint (2026-09-07)

`Check/Negative/Specifications.lean` retains all three original cases. Global
existence and modifies clauses reject non-Key types with exact, positioned
diagnostics. The old restriction on `old(address)` no longer applies: the
current value-level pre-state semantics proves it automatically, without
admissions. Two execution checks and canonical round-tripping also pass.
This is a test-only port; the new check passes at the unchanged cap and the
preceding 56/56 audit covers every other unchanged check, for **57 validated
Check files**. Source-port counts are **18/23 verification, 19/20 language,
4/9 negative**.

### Completed negative-surface checkpoint (2026-09-07)

`Check/Negative/Surface.lean` passes its eight diagnostic assertions, five
execution checks, and canonical round-trips. Move rejects duplicate active
loop labels (as compiler-v2 does); Rust can still shadow labels, and either
profile can reuse a label after its loop ends. A same-line operand after
`continue` is rejected rather than parsed as dead code and discarded.
The former positive Move label-shadowing fixture now uses distinct labels
and separately checks the required rejection. The complete capped Check audit
passes **56/56**: **18/23 verification, 19/20 language, 3/9 negative** source
ports. IR functional and composition-performance gates and Move/Rust suites
pass. The six recorded legacy performance regressions remain; no baseline or
budget was changed.

### Completed vector-operations checkpoint (2026-09-07)

The complete capped Check audit passes **55/55**:
**18/23 verification, 19/20 language, 2/9 negative** source ports.
`VectorOperations` passes all 33 original functions' structural checks,
25 verification requests with admission guards, 36 runtime cases, and canonical
round-trip checks at the unchanged cap. No failure baseline was added.
Work adds shared
native descriptors for swap/slice, value-level concatenation, range reversal,
empty destruction, membership/search, and explicit
Move source bounds failures (VM vector-operation error substatus 1, distinct
from library abort code 0x20000). Raw place operations remain the lower-level
unchecked vocabulary; canonical printing must preserve explicit guards.
The most recent focused `Check/VectorBounds.lean` run passed six proofs and
fourteen execution/state checks (including rollback and single evaluation of
computed indexes in both flat and nested vectors, direct writes, non-copyable
elements, RHS-before-index effects, and RHS-abort precedence),
admission guards, and canonical print/reimport at the unchanged cap. Pop-empty
tests VM substatus 2 in the larger port. The compact typed index check
avoids expanded guards and fixed-width length arithmetic. Nested checks use
the already-lowered place/index instead of evaluating the source again.

The capped 33-function run required reducing preparation and agreement costs. Batched
loan-death arena replacement lowered its certificate from about 87M to 36.5M
raw heartbeats. Exact-output tests against an independent sequential reference,
including a 32-function fixture, pass. Preparation sorting must
use structural recursion: the library's well-founded sort does not reduce in
the required reflexivity certificates. The isolated reversal proof now passes
50M, with literal-swap simplification keeping later reads/finalization compact.
The shared primitive certificate suite passes its 1M examples. Ordered sparse
place copies pass 256 comparisons against sequential updates and a symbolic
1M transitive-copy certificate. Queue-once reachability and kernel-friendly
arena lookup reduced the erasure-plan certificate from about 443M to 261M raw
heartbeats. A conservative local-type precheck reduced that to about 189M;
shared references nested in vectors, tuples, and mutable references retain
the full traversal and have explicit regression checks. A balanced proof-facing
index now reduces the traversal certificate to about 9.8M, plus 11.4M to build
the reusable indexes. Generic kernel-checked theorems connect index lookup to
array lookup and the indexed traversal to the unchanged native traversal.
Marking's kernel-friendly lookup reduced its certificate to about 22.6M.
Reusing the indexed expression results reduced expression rewriting/application
from about 23.8M to 3.5M. Complete preparation now passes its unchanged cap
without a retry (about 48.5M raw heartbeats from quotation through certificates).
Preparation retries reuse completed certificates, covered by an admission-free
regression. Retried preparation followed by the isolated `insert_middle` proof
now completes agreement and verification without admitted artifacts: rewriting
closed arena lookups to list lookups avoids repeated array-length scans. Its
typed contract proof costs about 14.45M heartbeats. All 25 typed vector
contract proofs complete individually (the largest is about 23.9M), and
the five remaining agreement failures are fixed by sharing balanced arena
indexes across generated proofs. The complete `VectorOperations.lean` run is
clean, not just the isolated proof runs, and is included in the full audit.

`Verification/LoopInvariants` has a fresh clean capped run: all three proofs,
admission guards, and six execution checks pass. The indexed mutation focus is retained, and retirement
certificates use explicit arguments to avoid duplicate elaborator unification.
The remaining bounds contradiction required normalizing both encoded-array
lengths and the constant `Int.natAbs` introduced by integer-to-natural casts;
negative conjunctions are split before arithmetic reasoning. `VectorBounds`
also passes its six proofs, fourteen execution/state checks, admission guards,
and round-trip checks. Direct indexed writes now check bounds even for literal
indexes; `core.assignPlace` preserves explicit guards across re-import. RHS
evaluation precedes destination evaluation, matching compiler-v2's mutation
lowering. Literal-only index bindings need no extra RHS temporary. Abrupt
initializers retain their declared type during validation and printing.
Preparation retry and the independent marker/copy/traversal regressions pass.
IR functional and composition-performance gates and Move/Rust suites pass.
The first audit exposed five regressions, all fixed and revalidated in the
second **55/55** audit: `ControlForms` selected a shadowed generated guard;
`EnumPatterns`, `EnumPayloads`, `Enums`, and `Generics` needed constructor-arity
lookup through shared preparation metadata. Now
normalization embeds exact local proof identities, and constructor certificates
unfold only the chain of shared unit metadata before the list-based arity
lookup. The duplicate-name regression passes at 1M.
The older IR `Performance` gate still reports its six recorded regressions
(`take_and_bump`, `scale`, `shift`, `reborrow`, `vector_get`, `vector_set`);
no cap or performance baseline was relaxed. This checkpoint does not claim
performance parity. The raw range-reversal primitive checks its complete
range; this port covers the original examples, not every degenerate
out-of-bounds behavior of a general standard-library wrapper.

### Completed generic-language checkpoint (2026-09-06)

`Language/Generics` now passes all sixteen original verification obligations
automatically under the unchanged 50M cap. The 25 original functions remain,
with seven concrete wrappers/observations: all 23 original runtime assertions
and sixteen added resource-family collision assertions pass. Structural checks
retain the actual resource-access kinds, generic binders, and concrete
publisher instantiations instead of V0's acquires/monomorphization metadata.
Printing and re-importing is a canonical fixed point. All sixteen public
proofs are checked for admissions; no failure baseline was added.

Generic call summaries now propagate native aggregate fields and returned
identity-coded vectors before continuation decoding. Boolean equality closure
normalizes only the relevant summary, not the executable preparation context.
Constructor equations are oriented toward known data, with the cycle guard
checking that direction. Concrete integer decoders consume their range
certificates before general equation search. These proof rules have 1M
regressions, including rejection of an invalid integer decoding.

Lowering completes only type instantiations demanded by operations and their
transitive callees, indexed by the reverse call graph. This supplies the missing
`Vault<u8>` key without adding a source declaration or cloning generic bodies.
Before the finite closure, parameter-flow SCCs reject growing instantiation
cycles using the VM's rule; identity cycles, permutations, and constant
substitutions remain legal. Tests also cover later-namespace callees and nested
resource keys. Logical scalar type inference refines from a later physical
aggregate argument, in either argument order, while incompatible physical
instantiations still fail.

The complete capped Check audit passes **52/52**: **18/23 verification**,
**18/20 language**, and **2/9 negative** source ports. IR functional targets,
`CompositionPerformance`, and the Move/Rust package suites pass. The generic
draft's monomorphic-summary dispatch regression and handwritten pilot-proof
contract mismatch are fixed. The older IR `Performance` gate still fails its
six recorded targets (`take_and_bump`, `scale`, `shift`, `reborrow`,
`vector_get`, `vector_set`); no cap or baseline was relaxed. This checkpoint
does not claim performance parity.

### Suspended generic-language draft checkpoint (2026-09-06, superseded)

This historical checkpoint was suspended at the user's request, not a
completed port; its remaining work is resolved by the checkpoint above. The new
`Check/Language/Generics.lean` retains all 25 original functions, 16 contracts,
and 23 execution assertions, with concrete execution wrappers and additional
type-family collision cases. Ten contracts currently verify: `identity`,
`box`, `unbox`, `swap`, `choose_generic`, `singleton`, `equal_generic`,
`equal_u64`, `wrap`, and `unwrap`. All 15 pure and eight original storage
execution assertions pass before the new collision fixture encounters its
missing `Vault<u8>` type. The raw generic ownership-domain regression passes.

Shared implementation changes in this draft:

- Generic enum twins now take native type parameters and certified codecs;
  roundtrip and tagged-payload rules are generated without assuming arbitrary
  codecs encode loan-free values.
- Move generic-dependent inputs explicitly require their actual encoded data
  to be loan-free. Native aggregate clauses encode at the runtime boundary;
  locals and parameters keep their native representations.
- Loan walkers consume the encoded input's ownership certificate. Native
  argument equalities simplify before erasures expand, avoiding reconstruction
  of the same generic input witnesses. Equality closure uses a single relevant
  codec/path fact rather than simplifying executable preparation hypotheses.
- Composition specializes the shared generic theorem at the actual invocation
  map. Its setup is extracted from the large generator to stay within the
  unchanged compiler limit; no proof cap or performance baseline was raised.

Remaining work, in order:

1. Fix generic summary/result propagation before continuation normalization.
   `equal_boxes`, `equal_choices`, `equal_vectors`, and `choose` hit the unchanged
   50M cap. `swapped` leaves an unresolved typed decoder; `singleton_length`
   leaves a residual return obligation. These are not accepted diagnostics.
2. Materialize resource types needed only inside instantiated generic callees.
   `tag_distinct` calls `tag_interactions<Bool, u8>`, but the lowered arena has
   no `Vault<u8>` node. `invocationTypeInstantiation` assumes those nodes already
   exist. Do not hide this gap by inserting an otherwise unnecessary source
   declaration of that concrete resource type. No lowering fix is implemented
   in this checkpoint.
3. Finish the structural replacements for v0's acquires/monomorphization
   assertions, source roundtrip, and the full Check/integration/performance
   audit. The five added collision scenarios are not yet validated.

`lake build LeanerLang` passes. Focused `SpecTypes` codec/data-certificate
tests and the existing native `Check/Verification/Generics` pass. The small
certificate regressions retain a 1M cap. The previous full Check audit and
six known performance failures below describe the preceding checkpoint,
not a fresh whole-suite result for this draft.

### Checked control expressions checkpoint (2026-09-06)

`Language/ControlForms` retains all 22 functions, 18 verifies (six with v0
hand proofs), and 37 executions. All verifies are automatic here. Two added
functions/contracts and six executions pin short-circuit OR, explicitly eager
core Boolean operations, and the VM's computed overflow payload for an index.
The complete port has 20 admission-checked proofs and 43 executions; source
render/reimport is a canonical fixed point.

Executable `&&`/`||` lower to branches, while explicit core primitives remain
eager, including through printing. Computed place indices are evaluated once
into typed locals before borrowing, reading, or assigning. Closed Boolean
and Boolean-tuple matches use decision trees with distinct-pattern coverage,
without enumerating the domain. Range increments use ordinary local writes,
and an entry guard keeps empty ranges outside the invariant abstraction while
still validating the body. No VM semantics or retired verification recognizer
changed.

Literal scalar preconditions simplify before execution normalization. Tuple
fallthrough closes by splitting only the scalar result's Boolean guard.
Boolean callee summaries now rewrite continuation guards directly: the
isolated call/match regression costs 14.22M typed-proof heartbeats rather than
exceeding 50M. These shared rules have 1M regressions; no cost baseline or cap
was relaxed.

The capped Check audit passes **51/51**, including clean individual reruns of
the two loop-containing files that exceeded the audit helper's wall-clock
timeout during overlapping high-memory diagnostic runs. The driver's 50M
heartbeat cap is unchanged. There are now **18/23 verification**, **17/20
language**, and **2/9 negative** source ports.

IR functional tests and `CompositionPerformance` pass. The older IR
`Performance` gate still fails only its six previously recorded targets:
`take_and_bump`, `scale`, `shift`, `reborrow`, `vector_get`, and `vector_set`.
Move and Rust package tests pass; Rust required the installed pinned
toolchain's `lib` directory in the command-local `LIBRARY_PATH` for linking.
This checkpoint does not claim performance parity.

### Structured language loops checkpoint (2026-09-06)

`Language/Loops` ports all fifteen v0 functions, ten contracts, and eighteen
execution checks. All ten contracts verify automatically, including nested
labeled exits/continues and resource-backed `drain`. The second `two_phases`
loop carries the `<= 3` bound from v0's hand proof. The separate v0
`Action`/`pure` and arrow-binding spellings map to ordinary LeanerLang locals;
its tail recurrence maps to a structured loop, not a helper or self call.
Structural assertions check those loop counts, absence of calls/helpers, and
the early return; source rendering/reimport is a fixed point.

Unannotated loops get the same typed header invariant without authored
clauses. A new negative case checks that this does not assume the function's
postcondition. Iteration-local slots remain optional and loan-free; other
out-of-scope slots are retained exactly from entry. This preserves the hidden
resource owner left by a field-borrow initializer. Scalar loops retain their
incoming location cache, with preservation proved as part of the invariant;
reference/call loops retain the general cache witness used by `clear`.

The surface now accepts `loop@outer`, `break@outer`, and `continue@outer`,
resolving the nearest enclosing label to the existing IR nesting depth.
Frontend regressions cover while/range nesting, shadowed labels, value-bearing
nonlocal breaks, unknown labels, and canonical print/reparse. No VM execution
semantics changed.

The capped Check audit passes **50/50**. The largest new typed proof is
`drain`: **30.18M heartbeats / 43,938 objects**; `two_phases` is 16.31M,
and the labeled countdowns are 13.66M and 13.38M. IR functional tests and
`CompositionPerformance` pass; the IR suite still fails only the same six
existing `Performance` targets recorded at the preceding checkpoint. Move
and Rust package tests pass. Caps and cost baselines are unchanged.

### Loop invariants checkpoint (2026-09-06)

`Verification/LoopInvariants` retains all three original contracts:
`count_to`, `sum_ones`, and mutation through a dynamically indexed vector
borrow in `clear`. All verify automatically, with no-admission guards and
six zero/multi-iteration execution checks. A new negative check rejects
invalid entry, preservation, and exit obligations; it is additional coverage,
not another port from the original nine negative files.

The generic computational loop boundary uses the checked native fixed point:
prove entry, one iteration preserving the invariant, and each exit. This is
partial correctness, not a termination proof. Live aggregate locals have
typed carriers; inactive slots are optional, loan-free values. Shared and
dynamic indexed borrows use checked focus rules. Active loan ownership is
preserved while allowing the VM's stale loan-location cache entries and
monotone loan counter. Vector decoding/re-encoding and immutable snapshots
normalize without traversing symbolic vector contents. Small shared-rule
regressions run under a 1M-heartbeat cap, including optional-slot collection
and updated vector decoding.

The full capped Check audit is **49/49**, including `References`; its interim
cap regression was fixed by classifying optional plain values before proof
search. IR functional tests and `CompositionPerformance` pass. The IR suite
still fails only its existing `Performance` cost gate (`take_and_bump`,
`scale`, `shift`, `reborrow`, `vector_get`, and `vector_set`); this checkpoint
does not claim performance parity. Move and Rust package tests pass.
No heartbeat caps or cost baselines were relaxed.

### Basic vectors checkpoint (2026-09-06)

`Language/Vectors` retains all six v0 verifies and executions, including
indexed reads/writes after a modular call and insertion/removal through a
mutable vector reference. The shared primitive vocabulary now includes
`insertVector` and `removeVector`; removal returns an element/vector pair so
the removed value is not silently discarded. Logical projection preserves
the physical vector representation while projecting the directly returned
scalar. Unit tests cover bounds, invalid operand/result types, serialization,
and normalized operation agreement.

The full capped Check audit is **47/48**, with only `LoopInvariants` failing.
The IR suite fails only its existing `Performance` cost gate;
`CompositionPerformance` passes. Move and Rust package tests pass; the Rust
source backend retains its explicit unsupported-vector-reshaping diagnostic
for the new operations. Caps and cost baselines are unchanged.
The next port is `LoopInvariants`, as requested, before other missing files.

### Structured declaration attributes checkpoint (2026-09-06)

`Language/Attributes` preserves nested resource-group annotations, enum
metadata, view/entry flags, numeric randomness metadata, dotted annotation
names, and both opaque and defined specification-function annotations.
`move_public` remains a public-visibility compatibility spelling without a
user annotation. The port uses `randomness = 7` for v0's positional literal.

User annotations are distinct from intrinsic owner/role graphs, including
when both occur on one declaration. Canonical rendering/reimport preserves
both; four negative cases pin malformed owners, missing owners, role arity,
and roles on nominal declarations. No new LIR metadata schema or verification
strategy is needed. Specification-function annotations use the existing
unified contract pragma slot and print as declaration attributes.

The full capped Check audit is **46/47**; only `LoopInvariants` remains a
failing draft. The IR suite still fails only its existing `Performance` cost
gate, with every verification target elaborating and `CompositionPerformance`
passing. Move and Rust package tests pass. Caps and baselines are unchanged.

### Loan composition and global-owner diagnostics checkpoint (2026-09-06, `cdb3313ea5`)

`Verification/Loans` passes all three original verifies and admission guards
under the unchanged 50M cap. `splice` consumes the proved `extend` contract
twice. Its second precondition failed because the arithmetic closer tried to
unify an unrelated vector-length bound with a multiplication lemma; reducing
that failed match expanded natural-number recursion toward the u64 limit.
The product certificate now requires an actual multiplication on the right.
Two 1M-cap regressions cover the original length failure and a valid product.

`Negative/BorrowGlobals` checks the original invalidation case at execution
preparation, including its exact conflict and related loan-origin diagnostic.
Global removal and publication now check active loans using the same family
identity as global borrowing. Three low-level tests cover shared/mutable
removal and publication; three positive source cases cover ended loans,
different families, and non-invalidating existence checks. Structural
`#leaner_unit` materialization alone is not an executable-safety check.

The full capped Check audit passes **45/46**, with only the existing
`LoopInvariants` draft failing. No positive failure baseline or increased
budget was accepted. The IR suite's only failure is its existing
`Performance` cost gate; all verification targets elaborate and
`CompositionPerformance` passes. Move and Rust package tests pass.

### Indexed-vector borrow checkpoint (2026-09-06, `160e81ebf3`)

`Verification/GlobalBorrows` passes all six original checks and admission
guards under the unchanged 50M cap. Shared certificates find and reconcile
an indexed loan in an otherwise loan-free symbolic vector without visiting
its prefix or suffix. Indexed place resolution rejects invalid indices and
wrong nominal tags before opening mutation machinery. Ten new unit checks
pass individual 1M caps, including vector-size facts and pure indexed reads.

Pure vector reads are normalized only in the arithmetic obligations that
need their bounds. Applying that exception globally regressed
`References.set_through_pair`; the scoped implementation restores its capped
pass. The full Check audit is **43/45**, with only `Loans` and
`LoopInvariants` failing. Neither caps nor failure baselines were relaxed.
`Loans.extend` and `Loans.independent_element` now pass using normalization;
the remaining composed `splice` precondition still needs work.
Move and Rust package tests pass. The IR suite's only failure remains the
existing `Performance` cost gate; every verification target elaborates and
`CompositionPerformance` passes. Performance baselines are unchanged.

### Nested-enum and negative-verification checkpoint (2026-09-06)

`Language/EnumPatterns` passes all six verifies, admission checks, and eight
execution assertions at the actual package-relative path under the unchanged
50M cap. The remaining timeout was specifically in
`nested_total.denotationRelation_agrees`, not its typed contract proof or
tree-denotation equality. Agreement construction now directly selects the
branch and enum-operation certificates instead of searching unrelated lemmas
and trying reflexivity on relational continuations.

The shared normalizer delays unselected computations for branching call-free
bodies. Straight-line bodies and modular callers retain the existing strategy:
applying lazy normalization indiscriminately regressed storage/invariant costs
and four checks, and that experiment was corrected before this checkpoint.
Six operation regressions cover same-owner tags, wrong owners, missing payloads,
and expensive unselected branches under a 1M cap. No semantics changed.

`Negative/Verification` preserves both original incorrect-contract cases and
checks that neither exports a verification theorem. Its baseline contains only
the intended clause-oriented rejections, not timeouts or unsupported features.

The full capped Check audit passes **42/45**, up from 40/44. The remaining
failures are `Verification/{GlobalBorrows,Loans,LoopInvariants}`; no previously
passing check regressed. Move and Rust package tests pass. The IR suite's only
failure remains its recorded `Performance` cost gate; its verification targets
now elaborate, and `CompositionPerformance` passes. No cap or performance
baseline changed.

### Boolean-witness and Enums checkpoint (2026-09-06)

`Language/Enums` now passes all five verifies and nine original interpreter
assertions under the unchanged driver cap. The mixed enum/Boolean matcher
costs **37.41M typed-proof heartbeats / 21,199 objects**, down from
51.68M / 25,474 (28% fewer heartbeats). All five verification theorems are
checked for missing declarations and admissions.

Call-free native proofs no longer enumerate Boolean witnesses while
simplifying the precondition: native argument equalities determine those
witnesses without duplicating representation work before the body branch.
Modular callers retain early Boolean decisions because their result decoder
currently needs the selected branch of conditional callee summaries.
This is a shared proof-generation change, not a body-layout row script.

The full Check audit passes **40/44**, up from 39/44, with no new failures.
The remaining four failures are `Language/EnumPatterns` and
`Verification/{GlobalBorrows,Loans,LoopInvariants}`. Move and Rust package
tests pass. The IR package retains only its previously recorded `Performance`
failure; `CompositionPerformance` passes. No cap or baseline was changed.
`EnumPatterns.nested_total` currently measures 65.36M typed-proof heartbeats
under a diagnostic ceiling and still does not qualify as ported.

### Bounded-vector and enum-payload checkpoint (2026-09-05, `408253f913`)

`Language/EnumPayloads` now passes all eleven original verifies and all ten
original execution assertions under the unchanged driver cap. Its thirteen
functions include nested calls, vector payloads, positional variants, and
mutable vector elements. `batch_length` measures 14.70M typed-proof heartbeats
/ 10,856 objects; `choose_and_score` measures 38.23M / 37,396.

Move arguments, results, locals, and twin fields use `SpecVector`, carrying
the same `< 2^64` length bound as v0. The boundary codec checks both element
decoding and length and has a proved roundtrip. Profile selection is explicit:
Rust and profile-neutral arrays are unchanged, as are execution semantics.
Arithmetic consumes the length certificate without traversing the vector.

At a call boundary, a generated enum decoder's own patterns recover its
native result. Constructor and tag equations discard incompatible variants
before the next call, and small pure-spec reductions normalize scalar summary
facts before range reasoning. No callee body is re-verified. Constructor
injection visits retained dependent equations only once, and never weak-head
normalizes unrelated semantic equations. Dedicated tests pin both properties
under a 1M raw-heartbeat cap; decoder and variant-pruning tests use 10M.

The old generated enum borrow-collection proof failed for bounded vector
payloads. It now uses the existing `Plain` theorem, and the check rejects
missing or admitted generated helpers and key verification theorems.

The full Check audit passes **39/44**, up from 38/44, with no new failures.
The five failures are `Language/{Enums,EnumPatterns}` and
`Verification/{GlobalBorrows,Loans,LoopInvariants}`. Move and Rust package
tests pass. The IR package has only the previously recorded `Performance`
failure; `CompositionPerformance` passes with its unchanged baseline.
Next is `Language/Enums.mixed_match`, proved diagnostically at 51.68M but
still above the unchanged cap. The `Enums` function census is corrected from
six to five, matching the v0 declarations.

### Literal-vector checkpoint (2026-09-05, `d6c97706bf`)

`Language/Literals` passes both original verifies and all seven execution
assertions under the unchanged driver cap. `classify_bytes` measures 13.90M
typed-proof heartbeats / 8,516 objects. Finalization uses the existing `Plain`
certificate to discharge the pruned borrow collector without inspecting a
symbolic vector's elements. The shared structural runtime comparison is now
proved lawful for every value constructor; vector guards remain equations
instead of being expanded into singleton-element witnesses. The closer uses
those same equations to discharge unreachable classification clauses.

New collector and equality regressions run under 5M and 1M raw-heartbeat caps.
The collector also has a negative regression containing a real borrow. Move
and Rust package tests pass; the IR package again has only its previously
recorded `Performance` failure, while `CompositionPerformance` passes unchanged.
The final full Check sweep passes **38/44**, up from 37/44. The six remaining
failures are language `EnumPatterns`, `Enums`, and `EnumPayloads`, plus
verification `GlobalBorrows`, `Loans`, and `LoopInvariants`. An intermediate
context-wide equality fallback regressed `CrossInv`; restricting it to vector
guards restores that check, and the final sweep introduces no new failures.
The next vector gap is representation: v0's `Move.Vector` carries a `< 2^64`
length certificate; Leaner's native array currently does not. This remains
necessary for `EnumPayloads.batch_length`, not a reason to weaken its clause.

### Returned-reference composition checkpoint (2026-09-05, `1e621f970d`)

`Check/Verification/References` now selects the compositional route. All twelve
checks pass the unchanged driver cap, including paired returned references and
projected global returns. Its loan-death assertions and deliberately false
returned-reference contract also pass. This completes the existing hand fixture;
it does **not** claim the larger v0 `ReturnedMutRefs` file is ported.

Shared boundary fixes cover mapped mutable-result decoding, projection-normalized
loan bounds, non-expanding summary equations, resolved sibling facts (including
literal resource decoders), returned-loan registration and global-registry facts,
and ordered reconciliation of any literal appended-write suffix. A staged
multi-loan retirement proof preserves inner-before-outer retirement without
prescribing a local layout. It is selected only when resuming a modular call;
the single-loan and non-call evaluators remain in place. Mutation shares its
cached lookup across direct and fallback certificates, and recognizes a stale
scalar lender hole without traversing its complete runtime place.

A separate measurement records `set_through_pair` at 51.25M aggregate typed-proof
heartbeats / 60,983 objects, down from 56.42M / 69,493 before these optimizations.
That aggregate includes work outside the generated command's cap; the actual
unchanged driver-cap run passes independently. The composition performance gate
passes without baseline changes (the four-call case is 51.24M versus its 48.55M
baseline, within its existing tolerance). Move and Rust package tests pass;
the IR package has only the already recorded `Performance` failure.

The complete Check sweep passes **37/44**, up from 36/44. Both EnumRefs files
remain green. The seven failures are `Language/{EnumPatterns,EnumPayloads,
Enums,Literals}` and `Verification/{GlobalBorrows,Loans,LoopInvariants}`.
No gate or diagnostic baseline was relaxed.

### Enum-reference verification checkpoint audit (2026-09-05, `96b613e06d`)

- All ten `Verification/EnumRefs` checks pass the unchanged driver's 50M
  per-command cap, including `scale`. Shared operation proofs certify the
  first matching resting borrower when the cached location is absent or
  stale; they preserve the evaluator's search order and do not specialize
  to a function's local layout. Negative tests retain the earlier nested
  borrower behavior. No executable semantics or budgets changed.
- `overwrite_three` now costs 36.58M typed-proof heartbeats and 28,685 proof
  objects, versus 42.25M and 34,534 at the execution checkpoint. Aggregate
  measurements are distinct from the driver's per-command cap: `scale`
  records 51.35M aggregate typed-proof heartbeats while its actual Check
  driver invocation passes.
- The complete 44-file Check audit passes **36/44**. The eight failures are
  `Language/{EnumPatterns,EnumPayloads,Enums,Literals}` and
  `Verification/{GlobalBorrows,Loans,LoopInvariants,References}`.
- IR tests pass except the older `LeanerLang.Tests.Performance` root.
  `OperationWP`, `Frontend`, `NativeStoreRow`, `NativeLocalCalls`, and
  `CompositionPerformance` pass. Move and Rust package tests pass. The
  independent Move/Rust source and Mono suites passed at the immediately
  preceding execution checkpoint; this proof-only checkpoint does not
  claim a new full E2E pass. No failure baseline was accepted.

### Enum-reference execution checkpoint audit (2026-09-05, `7f361d2be0`)

- `Language/EnumRefs` is complete: all sixteen functions and ten execution
  assertions pass. Its v0 source has no verification commands. The separate
  `Verification/EnumRefs` remains unported: nine of ten checks pass, but
  `scale` still exceeds 50M raw heartbeats.
- The compositional nominal path now preserves an optional enum variant tag.
  Its operation-level proof accepts arbitrary siblings and surrounding locals,
  not a particular function layout. The no-evaluator-fallback test covers
  arbitrary tags, and a negative test checks mismatched-tag rejection.
  `overwrite_three` now costs 42.25M typed-proof heartbeats. The remaining
  `scale` trace uses compositional borrowing but still spends several million
  heartbeats per general mutation/loan-retirement evaluation.
- The full 44-file Check audit still passes 35 files and fails nine: the
  same files listed below, with only `scale` failing inside `EnumRefs`.
  `leaner-ir` builds and its test suite fails only the older `Performance`
  root. `NativeStoreRow`, `NativeLocalCalls`, `CompositionPerformance`, and
  frontend tests pass. Move and Rust package tests pass too.
- `MoveStdlib/option` now passes the first-print/reimport fixed-point check.
  Shared pattern binders print as implicitly read fields; mutable binders
  remain explicit stable reborrows. A new seven-function Move fixture covers
  shared-reference return, shared value read, mutable-reference return, and
  mutable read/write, heterogeneous payload types, their concrete
  instantiation, and uniformly typed shared field names. Differently typed
  fields sharing a name retain their typed match, rather than becoming an
  ambiguous unqualified field projection. Its baseline, `option`, and
  `reflect` were regenerated only after successful fixed-point checks;
  `result` passes with its existing typed-match baseline unchanged.
- The rebuilt driver's complete Move-source suite passes, including all
  package fixtures. Rust-source, MonoVM smoke, and three-engine differential
  suites also pass independently. The full suite is still not green because
  Check has the nine failures above; no failure output was accepted as a
  passing baseline. The final source audit passes 35/44 files, and the IR
  rerun fails only `LeanerLang.Tests.Performance` with all native-call and
  composition gates unchanged and passing.

### Earlier follow-up worktree audit (2026-09-05)

- The 44-file Check sweep passes 35 files. The eight failures from the
  checkpoint below remain, and `Verification/EnumRefs` now also fails after
  correcting mutable enum-pattern lowering. The new `Language/EnumRefs`
  passes all ten execution assertions, but remains in progress while this
  integration regression is open.
- The final focused `Verification/EnumRefs` rerun fails only `scale` and
  `overwrite_three`; its other eight checks and generated path certificates
  pass. Static path proofs enumerate declared variants and discharge the
  unknown-name case through a shared checked absence lemma, avoiding whole
  namespace simplification. A focused mutable-payload read/write/abort
  contract in `Frontend` also passes at the unchanged 50M cap.
- `NativeLocalCalls` now passes every unchanged cost gate. Shared checked
  arithmetic agreement and integer-local borrow-freedom lemmas reduce
  `call_then_add` from 5.108M to 4.658M raw heartbeats, below its 4.943M
  ceiling; `add_then_call` costs 4.835M and `four_calls` 11.243M. These are
  boundary certificates; native computation VCs remain frame-free.
- `leaner-ir` builds and its full test run fails only the older `Performance`
  root. `NativeLocalCalls`, `CompositionPerformance`, and frontend
  round-trip regressions pass. The retired-route performance budgets were
  not adjusted.
- Fresh `leaner-move` and `leaner-rust` package tests pass.
- Printer round trips now hide only synthetic single-write reference
  holders, with a function-scoped reachability check. Copyable immutable
  enum matches print in the decision-tree form produced by source lowering.
  The successful `enums` expectation was regenerated, not a failure log.
  The full E2E run advanced past `aliased`, `enums`, and `big_ordered_map`,
  but still fails on the `MoveStdlib/option` reference-match fixed point.
  Later E2E driver suites are not claimed to pass.

### Commit checkpoint audit (2026-09-05)

This commit checkpoints the accumulated native-computation/composition work
and completes `Language/Integers`. It promotes four verification files
(`LooseFrame`, `Read`, `ResourceComposition`, `SpecLogicalArithmetic`) and
eight language files (`EmptyModule`, `Abilities`, `PositionalStructs`,
`Signed`, `Arithmetic`, `Tuples`, `Addresses`, `Integers`) to **ported**.
Their checks pass with `maxHeartbeats=50000` and
`weak.leaner.verifyHeartbeats=50000`: 50M raw heartbeats, not 50 billion.

- The complete 43-file Check audit matches expectations for 35 files.
  Eight fail: `EnumPatterns`, `EnumPayloads`, `Enums`, `Literals`,
  `GlobalBorrows`, `Loans`, `LoopInvariants`, and `References`. Drafts remain
  discoverable so those failures stay visible. `Enums` now exceeds the
  heartbeat cap; `Loans.splice` reaches its second composed call but exceeds
  the default recursion-depth limit. Earlier passing measurements in the
  historical notes below do not supersede this audit.
- `Integers` passes all 12 verifies and 31 interpreter assertions. Shared
  descriptors have evaluator-agreement proofs for casts, bitwise operations,
  and shifts; nested primitive constants fold with checked agreement rather
  than becoming assumptions. Specification shifts retain VM masking behavior.
  `NativeCasts` passes at 5M raw heartbeats; `NativeConstants` also passes.
- `leaner-ir` builds. Its full test suite passes except the cost gates
  `Performance` and `NativeLocalCalls`. The latter's `call_then_add` costs
  5.108M aggregate raw heartbeats against the unchanged 4.943M ceiling; its
  proof-object count remains below budget. The older performance gate still
  has over-budget targets awaiting migration. `CompositionPerformance` passes.
- Fresh `leaner-move` and `leaner-rust` `lake test` runs pass.
- Full `leaner-e2e-tests` `lake test` fails first at the Move printer's
  `aliased` canonical fixed-point check: re-lowering a field assignment adds
  a temporary reference binding, so the second print differs. This is an
  integration regression, not an accepted baseline change. The independent
  Check audit above runs despite that earlier driver failure; later full
  driver suites are not claimed to have completed.

No failure baseline, heartbeat cap, recursion-depth limit, or recorded
performance budget was relaxed. This is a committed progress checkpoint,
not the completion of T1 or an all-green integration milestone.

### Historical current-tree audit (2026-09-04)

Use the driver's actual options, not a larger substitute:

```bash
cd leaner-e2e-tests
lake env lean -DmaxHeartbeats=50000 -Dweak.leaner.verifyHeartbeats=50000 \
  LeanerE2ETests/Check/Verification/GlobalBorrows.lean
```

Lean's option counts thousands of raw heartbeats: `50000` is the 50M
cap. `50000000` would silently allow a thousand times more work.

- Eleven of the twelve previously landed source ports match their expected
  diagnostics; `GlobalBorrows` has the cap regression recorded above.
- `Aborts.exp` intentionally contains the older wrong-increment negative
  check. The five `AbortDirections` targets in that file pass. The previous
  statement that none of the twelve ports had an `.exp` was incorrect.
- The audit exposed, and the current work repairs, scalar-twin generation,
  monomorphic/recursive call selection, concrete storage-key normalization,
  and stale `EvalFunction` test signatures after adding invocation type maps.
- Outside those twelve ledger rows, `Generics` and `References` also have
  regressions against their existing check expectations. They must not be
  hidden by accepting new failure baselines.
- The performance benchmark is **not passing**. In particular, generic
  scalar-result callers remain unsupported and several existing cost classes
  exceed their recorded budgets. `Performance.exp` has not been relaxed.
- Validation of the current work: `leaner-ir` builds and its unit-test
  targets pass except `LeanerLang.Tests.Performance`; `leaner-move` and
  `leaner-rust` pass `lake test`. The capped Check audit leaves `Generics`,
  `References`, and `GlobalBorrows` failing their existing expectations.

A new file's existence, a run with a larger cap, or an expected diagnostic
for an unimplemented route is not evidence that a source port is complete.

### Follow-up audit (2026-09-05, uncommitted work)

A 30-file capped Check sweep plus targeted reruns after repairs matches
expectations for 26 files. In particular, `GenericStorage` again passes all
seven checks: the caller consumes the callee's Boolean equivalence before
storage-key normalization splits away its matching connective. `Storage`,
`Normalized`, `GlobalInv`, and `CorePrimitives` also pass after repairing
Boolean result decoding and contextual-keyword parsing. Neither diagnostic
baselines nor the driver cap were changed.

The four failing files are `Generics`, `References`, `GlobalBorrows`, and
the newly drafted `LoopInvariants`; their gaps remain explicit, not accepted
failure baselines. At that audit, `LooseFrame` and `SpecLogicalArithmetic`
passed individual port checks but were uncommitted and awaiting broader gates.
The new one/two/four-call composition cost gate passes
(latest four-call sample: 48.75M heartbeats, 44,182 proof objects), but the
older performance gate still fails. These results do not supersede the need
to rerun downstream package suites after the source changes settle.

The subsequent scalar-call checkpoint adds `GenericScalarCalls.lean`: all
ten checks pass at the same cap, including both strict/omitted and explicit
abort clauses, and concrete integer, Bool, address, string, signer, and byte
results. Shared canonical-decoder shape lemmas replace per-call runtime
constructor splitting; malformed empty-family proof quotations are repaired.
`GenericStorage` still passes all seven checks. Fresh `leaner-move` and
`leaner-rust` `lake test` runs pass, as do all IR test targets except the old
performance gate. That gate now has no failed verification targets: its
remaining failures are costs. In particular, `carry_u64` is 7.36M typed
heartbeats against its 4.76M baseline (54% over), so it is functionally
repaired but not performance-complete. Existing cost baselines remain intact.

**Native-computation pilot (2026-09-05):** work has shifted from tuning row
scripts to replacing their verification path. New IR/LeanerLang rule tests
cover exact typed-computation transport, generic representation edge cases,
and fail-closed route selection. The explicit-artifact `carry`/`carry_u64`
pilot passes an aggregate cost gate against the unchanged original ceilings,
including its agreement proofs and transports (approximately 1.44M and 3.40M
heartbeats). This is infrastructure work, not a newly ported v0 file:
automatic native-body generation and migration of the original performance
targets are still pending. No port status is promoted by this checkpoint.

**Automatic native forwarding (2026-09-05):** the original `Performance`
`carry` and `carry_u64` now use generated native computations and pass their
unchanged cost limits, including agreement/helper proofs and transports.
The aggregate caller cost is 2.94M heartbeats versus the original 4.94M
ceiling. `NativeGenerated` adds six positive forwarding/call checks, three
negative precondition/postcondition checks, failed-artifact rollback checks,
and dependency audits excluding execution frames from native VCs. The full
IR suite still fails only the cost gate on other, not-yet-migrated targets;
fresh Move and Rust suites pass. This remains infrastructure work: it does
not promote any v0 file's port status. Native effects, references, storage,
loops, and direct native contract generation remain pending.

**Native checked arithmetic (2026-09-05):** the original `guarded` performance
target and `SpecLogicalArithmetic.checked_successor` now use generated native
checked arithmetic, without frame/row VCs. `guarded` passes its unchanged
cost limits with 54% fewer aggregate heartbeats and 45% fewer proof objects,
including agreement and transport. New infrastructure tests cover nine
positive generated cases, three rejected contracts, failed-artifact rollback,
dependency audits, and seven signed/unsigned arithmetic boundary cases. The
source `SpecLogicalArithmetic` check stays clean at the driver cap; its other
two branch/comparison verifications have not yet migrated. This does not
promote a v0 file's port status. Native sequencing and aborting-call summaries
are next; the remaining old-route performance failures are still visible.

**Native aborting calls (2026-09-05):** `NativeCalls` adds six positive
modular callers and five rejected contracts, including an abort property
true of the callee implementation but absent from its published contract.
Generated caller proofs reuse native summaries, without execution frames
or semantic effect/agreement certificates. Rollback checks include the
reusable registry. All generated stages and public transport fit the
unchanged original call-cost ceiling: 2.96–4.62M heartbeats and 3,052–5,006
objects per caller, versus 4.94M and 8,126. A core rule regression covers
two opaque calls where the second aborts. Automatic source/local sequencing
is still pending; this is infrastructure coverage, not a new v0 file port.
Fresh Move/Rust suites and capped `GenericScalarCalls`, `GenericStorage`,
and `SpecLogicalArithmetic` checks pass. The full IR suite fails only the
existing performance gate on unmigrated targets; all new native-call tests
and the already migrated original cost targets pass. Baselines are unchanged.

**Native typed locals (2026-09-05):** `NativeSequence` covers checked
addition/subtraction through typed local bindings, saved/discarded values,
and a modular caller of a sequenced callee. Ten positive and four negative
checks pass, along with exact initializer/continuation failure-payload tests,
rollback and dependency audits. All generated stages and transport cost
3.79M/6.92M/17.30M heartbeats for two/four/eight operations, and 3.61M for
the modular caller. Unchanged original budgets and additional chain-scaling
gates pass. Call-initialized locals, nested arithmetic operands, mutable
assignments, heterogeneous locals, and other effects remain pending. No v0
file's status is promoted by this infrastructure checkpoint.
Fresh Move/Rust suites and the capped generic-scalar, generic-storage, and
logical-arithmetic source checks pass. The full IR suite continues to fail
only the older cost gate on unmigrated targets, with baselines unchanged.

**Native call-initialized locals (2026-09-05):** `NativeLocalCalls` adds ten
positive caller checks, six rejected contracts, exact initializer versus
continuation error checks, and rollback/dependency audits. Caller proofs
reuse native summaries, including a sequenced callee and a precondition
established by an earlier call. Call→arithmetic and arithmetic→call cost
4.82M and 4.87M aggregate heartbeats; two/four calls cost 5.99M/11.11M.
Unchanged original budgets and the repeated-call doubling gate pass. Fresh
Move/Rust suites and selected capped source checks pass; the full IR suite
still fails only the older cost gate. No v0 file is promoted: nested operands,
generic/heterogeneous call bindings, assignments, and other effects remain
pending infrastructure work.

**Native nested operands (2026-09-05):** `NativeOperands` covers nested left,
right, and both-side checked addition/subtraction, typed local initializers
and results, and sequencing with modular calls. Eleven positive verifies,
five rejected contracts, artifact/registry rollback, dependency audits, and
three exact error-payload assertions pass. Two/four nested additions cost
4.53M/8.04M aggregate heartbeats and 5,521/7,646 objects, including execution
agreement and public transport; the unchanged budgets and doubling gate pass.
The full IR suite still fails only the inherited `Performance` cost gate.
Nested call operands, heterogeneous/generic bindings, assignment, storage,
and loops remain outside this native expression generator.

The source audit exposed the old frame-entry proof regression in
`Check/Verification/Generics`. Its four verifies now use the native route and
pass the driver cap. Brittle `#check` output was replaced by checked type
examples, including the explicit type-instantiation argument; the obsolete
print-only `.exp` is removed. This is a repaired existing fixture, not an
additional v0 verification-file port. The complete source suite is not yet
claimed green.

**Complete source audit (2026-09-05, after the generic repair):** every
existing source fixture was run independently with the driver's exact
options. Only `GlobalBorrows`, `LoopInvariants`, and `References` differ from
their expectations. The first exceeds 50M heartbeats; the second lacks
Boolean negation/vector-length descriptors and an accumulator-loop route;
the third has generated field-binder and storage-frame proof errors.
`Loans`, `Read`, `ResourceComposition`, `LooseFrame`, and all previously
passing source fixtures match. The newly added enum source subsequently
passes too. These are per-file results, not a claim that the source driver
or legacy performance gate is green.

The enum port adds checked raw-payload decoder equations and a closed
constructor-field lookup proof (array search is not `rfl`-reducible).
Specification equivalences normalize literal enum data before introducing
directions, so impossible variants remain visibly false. Bad tags, payload
types/counts, owners, and out-of-range payloads are rejected by codec tests.
The tight native local-call performance gates still pass.

**Descriptor and boundary audit (2026-09-05):** Boolean negation and length
now have native descriptors with exact evaluator-agreement theorems. Kernel
tests cover invalid operand rejection, UTF-8 byte length, fixed-width length
wrapping, and unbounded length. The capped loop rerun progresses past the
missing descriptors to the route/loan gaps recorded in its ledger row.
This does not claim that native loop computations or VCs have been generated.

`CheckSupport.assertRuns` executes the registered validated unit directly;
the enum and positional-struct ports retain their original interpreter
assertions. It currently checks outcomes, not final stores. Resource ports
must also check state before using it to replace v0's state assertions.

Two semantic issues surfaced while examining subsequent ports:

- v0's `Move.Vector` carries a proof that its length is below `2^64`
  (`move/Move/Basic.lean`). Current native vector codecs expose an arbitrary
  `Array`, while bounded execution length uses modular conversion. A proof
  that an arbitrary array length fits would be unsound. The bounded native
  representation and growth semantics must be reconciled before claiming
  exact vector-length contracts for arbitrary inputs.
- v0 `Language/Signed.mod_values` says remainder aborts only when the divisor
  is zero; its specification and old interpreter return zero for
  `(-2147483648) % (-1)`. The real Move VM's `rem_checked` calls
  `i32::checked_rem`, which rejects this pair. Current Leaner checked modulo
  also rejects it, with the out-of-range quotient payload `2147483648`.
  New kernel tests pin this exact result and the successful neighboring
  `MIN_INT % 1` case. Porting the original contract unchanged would require
  disagreeing with actual execution; correction of the port's abort clause
  was approved by the user: **match the VM**. The new source port includes
  the extra remainder-overflow case and retains all other original checks.
  Its twelve interpreter assertions pass, including zero divisors, both
  signs, the overflow pair, and its successful neighbor. All six verifies
  pass under the unchanged cap. The constructive closer now extracts signed
  certificate bounds, proves a failed range check as one disjunction before
  selecting either endpoint, and normalizes storage-valued disjunctions before
  splitting them. Neither v0 nor production VM semantics was changed.

## Where existing tests go

- `LeanerLang/Tests/Verification*.lean` (46 verifies): each becomes a
  check under `Check/Verification/` and the fixture is deleted once its
  check lands; the negative ones (`VerificationAborts`, `reborrow_bad`)
  land with their `.exp`.
- `LeanerLang/Tests/Performance.lean` and `.exp`: unchanged; it is the
  cost gate and stays a curated set.
- The hand row proofs (`NativeRow`, `NativeCallRow`, `NativeStoreRow`):
  unchanged; they test proof rules, not programs.
- Assertion-style tests in `leaner-move`, `leaner-rust`, and
  `LeanerIR/Tests`: unchanged.
- The deprecated packages stay until the ledger is complete.

## Milestones

| Status | Step | Gate |
|---|---|---|
| **DONE (2026-09-02)** | **T0 the check driver.** `LEANER_E2E_SUITE=check` (`LeanerE2ETests/Check.lean`): discovers `Check/**/*.lean`, runs each in its own `lean` process under the heartbeat cap, records its output verbatim, compares against the optional `.exp` through `Baseline.checkOutput` (`UB=1` writes or removes, as compiler-v2 does). The cap reaches the generated theorems through the registered option `leaner.verifyHeartbeats`, which the generator reads in place of its former literal budget (default unchanged); the driver passes it as `-Dweak.leaner.verifyHeartbeats`. Seeded with `Check/Verification/Increment.lean` (clean, no `.exp`) and `Check/Negative/WrongIncrement.lean`, whose `.exp` is lean's two diagnostics: the clause not established at its range, and the module's verification failure. | Met: the suite runs in `lake test`; the positive seed has no `.exp`, the negative seed's `.exp` is the verbatim output; a deliberately wrong clause on the positive seed produced a baseline diff, and a budget of 10 produced the deterministic timeout at the `verify`. |
| **IN PROGRESS (since 2026-09-02)** | **T1 port the v0 trees.** One file at a time, ledger kept current. Native-encoding migration now takes priority over additional ports. Historical recommended order: `Verification/` first (automatic targets before proof-carrying ones; `Quicksort` and `OrderedMap` last), then `Negative/`, then `Language/`. The last committed checkpoint has eighteen verification source ports, nineteen language files, and seven negative files passing its Check audit; active worktree regressions are recorded above. Incomplete and regressed rows are marked explicitly above; no new failure baseline qualifies a file as ported. The latest committed checkpoint completes `Negative/Lowering`; the preceding hand `References` fixture is not the v0 `ReturnedMutRefs` port. See the latest audit for integration and performance failures. Historical benchmark numbers are not current passing measurements. | Every ledger row passes its check under the driver cap, and the performance gate passes. Parity target: every v0 automatic verify is automatic here; proof-carrying targets may be automatic or use explicit proofs of their mathematics. |
| NOT STARTED | **T2 the produced LeanerLang verifies.** The Move stage appends `verify f` for every spec'd function with a body (v0's `--verify`), so the `.exp.lean` baselines carry the commands, and every `.exp.lean` under `MoveToLeanerLang` runs as a check with `<module>.check.exp` beside it. Most stdlib rows will read as failures at first: that is the benchmark plugged into the ground. Same later for `RustToLeanerLang` once Rust contracts exist (rust-mir-design M2). | Every produced module with a spec has a check result; a `proved` that turns into anything else fails the suite. |
| **DONE (2026-09-02)** | **T3 retire the hand fixtures.** The ten `LeanerLang/Tests/Verification*.lean` fixtures are checks under `Check/Verification/` (`Prophecies`, `Aborts`, `Storage`, `Generics`, `Corpus`, `References`, `Loops`, `Typed`, `Rust`, plus the seeds `Increment` and `Account`); their `.exp` files record the constructs without a generated route, since the frame route was retired the same day. | Met: `leaner-ir`'s verification tests are the row proofs and the cost gate only. |

T0 and T2 are driver work with no proof engineering and are independent
of the route work in [`certifying-execution.md`](historical/certifying-execution.md);
they are what makes that work measurable.  T1 is where the frontends'
and routes' gaps become ledger rows.

## Deliberately not in scope

- A Move Prover differential.  The parity claim is against v0's recorded
  verdicts and proofs, not against Boogie's.
- Proving compiler correctness for the emitted bytecode; the claim is
  "the same verdict on the same specification", per the CLAUDE.md
  separation of claims.
- Deleting the deprecated packages before the ledger is complete.
