# Test organization and current native migration status

Updated 2026-09-08. The last full audit covers literal scalar-owner mutation;
the subsequent checked-expression work is unfinished, as recorded below.
These are **partial validation results recorded in a checkpoint commit**, not
evidence that the migration is complete. This document is the current
ledger; old milestones and measurements are in
[the history archive](test-organization-history.md).

**“Source checkpoint” is a retired historical label, not a completion status.**
It meant a source translation was committed and passed at that earlier point,
possibly using legacy encoding. **Yes, those files can still contain unresolved
problems today.** Use the explicit pass/fail rows below, not that old label or
the existence of a commit, to determine what remains.

Read [the audit summary](#current-result) for totals,
[unfinished changes](#unfinished-changes-after-the-last-full-audit) for newer
failures, and [per-file status](#per-file-status) for the remaining work in each
acceptance fixture. Implementation details and test-running rules follow the
ledger.

## Current result

**Last full audit: 19 of 61 existing Check files passed; 42 failed. Migration is not finished.** Of the failures,
39 files contain native implementation/proof gaps and three have diagnostic
baseline mismatches. These are file counts, not counts of independent bugs.
Seven additional v0 files still have no acceptance fixture (listed below).

Read the per-file tables for test failures, the gate table for package and
performance failures, and the implementation section for what the latest
batch actually supports. A passing pilot does not promote an acceptance file.

Legacy verification fallback is **off**. Retired route selections reject,
including cached-proof reuse; every successful `verify` audits its generated
native computation and verification-condition dependencies. Unsupported cases
fail rather than falling back. Native verification does **not** yet mean all
legacy code is gone: compatible `RuntimeState`, failure, and contract adapters
remain. See [denotation](denotation.md) for the current design; the native route's boundary is recorded in [generic-route](historical/generic-route.md).

### What the labels mean

| Status | Meaning in the last full audit | Is there unfinished work in this file? |
|---|---|---|
| PASS | The entire current fixture matches its expected output at the unchanged cap, with fallback disabled. Successful verifies pass the automatic native audit. | No failure in this audit; coverage is limited to the assertions in the file. An execution-only or diagnostic-only pass is not a verification port. |
| FAIL — native gap | At least one intended positive verification or required proof is unsupported or fails. Other targets may already pass natively. | **Yes.** The row names known gaps; it is not promoted by partial success. |
| FAIL — diagnostics | Intended negative cases reject, but their diagnostics differ from the checked-in baseline. | **Yes.** Review the rejection reason and diagnostics before updating expectations. |
| MISSING | No corresponding current Check fixture exists. | **Yes.** Translate the v0 cases and implement their native support. |

The historical 18/23 verification, 19/20 language, and 7/9 negative counts
are **not current completion counts**. Likewise, the old 60/61 mixed-route
result is not native coverage.

### Unfinished changes after the last full audit

Checked expressions in mutable assignments (`*slot := *slot + 1`, division,
and multiplication) are being implemented. They are **not accepted as a
completed batch**:

| Check | Latest observed result | Remaining work |
|---|---|---|
| Production `LeanerLang` build | PASS | Compilation alone does not establish the source proofs. |
| `NativeMutableGenerated` extended regression | PASS (focused rerun) | The bounded `square` proof is fixed by reducing operand tuple projections before arithmetic reasoning. All 14 positive targets, four false-contract rejections, existing execution assertions, and artifact guards pass. Maximum cost is 8.410M heartbeats / 10,115 objects, below the unchanged 10M/15k ceiling. Additional checked-update execution coverage and broader gates remain pending. |
| Full acceptance and package gates for these changes | NOT RERUN | The results below belong to the earlier literal-assignment batch, not these newer edits. |

Evidence: `/tmp/dev3-native-square-fix-build.log` and
`/tmp/dev3-native-square-fix-test.log`. These supersede the earlier failing
square regression. No acceptance file is promoted by these partial results.
The current worktree is **not fully validated**. Work is suspended at the
user's request after this focused fix. The subsequent checkpoint commit records
the unfinished work without promoting any acceptance status.

### Other required gates — last full audit, before unfinished changes

These results include automatic scalar-owner mutation. The full IR suite still
fails; its failures are separate from the passing native regression subset.

| Gate | Last completed batch result | Remaining problem |
|---|---|---|
| IR/native regression subset | PASS (200 build jobs) | All registered roots except the six explicitly failing targets below. Includes boundary and generated mutation tests. |
| Native cost gates | PASS | Existing budgets unchanged. Nine generated mutable targets cost 3.28–4.23M heartbeats and at most 5,384 objects across all stages, within the reference-test 10M/15k ceiling. The tight prepared-call gate remains 9.923M/11,631. Shared boundary kernels retain their 1M caps and selected 10k-object guards. |
| Full IR test target | FAIL | Six targets: `NativeRow`, `NativeCallRow`, `NativeStoreRow`, `CompositionPerformance`, `Performance`, `Frontend`. Not dismissed by the native unit passes. |
| Move tests | PASS (71 build jobs) | None observed in this run. |
| Rust tests | PASS (165 build jobs) | None observed in this run. |

A completed port requires the intended v0 cases, native proofs, execution and
negative assertions, admission/agreement checks, and unchanged performance gates
to pass. A commit is recorded separately; it cannot change a failing status.
The overall migration is not complete while these failures remain.

## Per-file status

Paths below are relative to
[`leaner-e2e-tests/LeanerE2ETests/Check/`](../leaner-e2e-tests/LeanerE2ETests/Check/);
each name denotes a `.lean` file. All 61 existing files appear exactly once.
The problem column summarizes observed failures and known missing support,
not a claim that fixing the first error will finish the file.

### Language (19 files: 8 pass, 11 fail)

| File | Status | Passing coverage / remaining problem |
|---|---|---|
| `Language/Abilities` | PASS | Declaration and ability metadata; no verification targets. |
| `Language/Addresses` | FAIL — native gap | Resource reads and reference locals/calls remain unsupported. |
| `Language/Arithmetic` | FAIL — native gap | Ten scalar targets pass; resource-reference `multiply` and `divide` do not. |
| `Language/Attributes` | PASS | Declaration metadata, diagnostics, and round trips; not a verification port. |
| `Language/ControlForms` | FAIL — native gap | Fifteen targets pass, including conditional assignment and checked assertions; reference/tuple and other unsupported body shapes remain. |
| `Language/EmptyModule` | PASS | Empty-module elaboration/preparation; no verification targets. |
| `Language/EnumPatterns` | PASS | All six verified targets and interpreter assertions. |
| `Language/EnumPayloads` | FAIL — native gap | Unsupported value and checked-arithmetic shapes remain in payload/vector cases. |
| `Language/EnumRefs` | PASS | Execution-only: sixteen functions and ten execution assertions, **zero verifies**. |
| `Language/Enums` | PASS | All five verified targets and interpreter assertions. |
| `Language/Generics` | FAIL — native gap | Generic aggregates/results, resource operations, and vector type metadata remain unsupported. |
| `Language/Integers` | PASS | All twelve verified targets, including the shared-reference case, and interpreter assertions. |
| `Language/Literals` | FAIL — native gap | `fixed_address` passes; vector classification does not. |
| `Language/Loops` | FAIL — native gap | Ten targets in the original module and two return-scope regressions pass native audits. Only `drain` remains a verification gap: resource/reference loop state. |
| `Language/PositionalStructs` | PASS | Both verified targets and interpreter assertions. |
| `Language/Signed` | FAIL — native gap | Five scalar arithmetic targets pass; resource-reference `credit` does not. The port must retain VM-correct remainder overflow. |
| `Language/Tuples` | FAIL — native gap | Tuple parameter/result and local-sequencing representations remain unsupported. |
| `Language/VectorOperations` | FAIL — native gap | `borrowed_length`, `nested`, `bool_round_trip`, `read_out_of_bounds` pass; remaining operations include mutation and unresolved value/type shapes. |
| `Language/Vectors` | FAIL — native gap | `make`, `length`, `middle` pass; `replace`, `insert_middle`, `remove_middle` do not. |

### Verification (30 files: 7 pass, 23 fail)

| File | Status | Passing coverage / remaining problem |
|---|---|---|
| `Verification/Aborts` | FAIL — native gap | Six state-free targets pass; resource-backed `withdraw` does not. Intentional false-contract rejection is separate. Includes the v0 `AbortDirections` port. |
| `Verification/Account` | FAIL — native gap | Resource-effect function shapes remain unsupported. |
| `Verification/BorrowCertificates` | PASS | Certificate assertions, not source verification targets. |
| `Verification/Callees` | FAIL — native gap | Reference locals, unavailable native callee summaries, and recursive/effectful cases remain. |
| `Verification/Calls` | FAIL — native gap | Reference locals and unsupported parameter/result/effect shapes remain. |
| `Verification/Composition` | FAIL — native gap | Native callee availability and supported call-argument shapes remain incomplete. |
| `Verification/CorePrimitives` | FAIL — native gap | Unsupported effectful body shapes and local arithmetic remain. |
| `Verification/Corpus` | FAIL — native gap | Reference locals and unsupported effectful function shapes remain. |
| `Verification/CrossInv` | FAIL — native gap | Cross-resource invariant bodies need native reference/storage effects. |
| `Verification/EnumRefs` | FAIL — native gap | Selected-variant payload handling and reference locals remain incomplete. |
| `Verification/GenericScalarCalls` | PASS | Ten verified targets and eight generic-summary reuse guards. |
| `Verification/GenericStorage` | FAIL — native gap | Generic storage operations and their value representations remain unsupported. |
| `Verification/Generics` | PASS | Four existing native proofs and native-artifact checks; not the broader `Language/Generics` fixture. |
| `Verification/GlobalBorrows` | FAIL — native gap | Resource/reference effects and reference locals remain unsupported. |
| `Verification/GlobalInv` | FAIL — native gap | Resource mutation/invariant bodies and value representations remain unsupported. |
| `Verification/Increment` | PASS | Positive verification seed. |
| `Verification/Invariants` | FAIL — native gap | Native arithmetic/input handling, enum invariant closing, and resource-effect cases remain. |
| `Verification/Loans` | FAIL — native gap | Native loan/reference effects and composition remain incomplete. |
| `Verification/LoopInvariants` | FAIL — native gap | `count_to` and `sum_ones` pass; `clear` needs mutable vector/reference loop state. |
| `Verification/Loops` | PASS | `count_to` and `count_to_with_continue` pass with native audits. |
| `Verification/LooseFrame` | FAIL — native gap | Native resource-effect bodies remain unsupported. |
| `Verification/Normalized` | FAIL — native gap | Reference locals and unsupported value representations remain. |
| `Verification/Prophecies` | FAIL — native gap | Reference locals remain unsupported. |
| `Verification/Read` | FAIL — native gap | Reference-local sequencing remains unsupported. |
| `Verification/References` | FAIL — native gap | Returned-reference representations and reference/effectful function shapes remain. This is **not** the v0 `Verification/ReturnedMutRefs` port. |
| `Verification/ResourceComposition` | FAIL — native gap | Native sequential resource-write composition remains unsupported. |
| `Verification/Rust` | FAIL — native gap | Reference-local and effectful function shapes remain unsupported. Separate from the passing Rust package gate. |
| `Verification/SpecLogicalArithmetic` | PASS | Current logical-arithmetic verification/assertions pass. |
| `Verification/Storage` | FAIL — native gap | Native reference/storage effects and value representations remain unsupported. |
| `Verification/Typed` | PASS | Native struct identity verification; expected `#check` output matches. Typed compatibility adapters still appear in interface assertions. |

### Negative and support (12 files: 4 pass, 8 fail)

Negative fixtures can contain positive controls. A correct rejection does not
make the file pass if its intended positive proof fails.

| File | Status | Passing coverage / remaining problem |
|---|---|---|
| `Negative/BorrowGlobals` | PASS | Borrow-safety rejection and positive execution controls. |
| `Negative/Borrows` | FAIL — native gap | Positive reference-local proofs fail; not merely a diagnostic mismatch. |
| `Negative/IntrinsicUnsupported` | PASS | Schema/lowering rejection and validation controls; not native-map execution support. |
| `Negative/LoopInvariants` | FAIL — diagnostics | Incorrect invariant contracts reject through native generation; diagnostic text/reasons still need review against the baseline. |
| `Negative/Lowering` | FAIL — native gap | Positive resource/value-representation cases fail, including missing `receiver_insert` proof. |
| `Negative/ReturnedMutRefs` | FAIL — native gap | Local/global-root return rejections are retained; valid parameter-derived reference proofs still lack native support. |
| `Negative/Specifications` | FAIL — native gap | The positive old-value proof fails on reference-local sequencing. |
| `Negative/Surface` | PASS | Intended surface rejections, execution controls, and round trips. |
| `Negative/Verification` | FAIL — diagnostics | Wrong contracts reject; native failure diagnostics differ from the baseline. |
| `Negative/WrongIncrement` | FAIL — diagnostics | Wrong increment rejects; native failure diagnostics differ from the baseline. |
| `PreparationRetry` | PASS | Preparation/retry assertions and the constant function's native verification. |
| `VectorBounds` | FAIL — native gap | `read_bad` and `read_reference` pass; mutable indexing and remaining value/local shapes do not. |

### Missing v0 fixtures (outside the 61-file audit)

| v0 file | Status | Remaining work |
|---|---|---|
| `Verification/OrderedMap.lean` | MISSING | Port nine proof-carrying verification targets and their dependencies. |
| `Verification/Quicksort.lean` | MISSING | Port three proof-carrying verification targets and their dependencies. |
| `Verification/ReturnedMutRefs.lean` | MISSING | Port the 66-target verification corpus; the hand `References` and negative fixture do not replace it. |
| `Verification/SpecFunctions.lean` | MISSING | Port twelve verification targets. |
| `Verification/Summaries.lean` | MISSING | Port three verification targets. |
| `Negative/SpecFunctions.lean` | MISSING | Port the diagnostic cases. |
| `Language/BorrowChecker.lean` | MISSING | Port the language/borrow-checker cases. |

## Last completed implementation batch and next work

The last completed batch automatically emits a typed ownership-output body,
authored native contract, exact execution agreement, and checked boundary
transport for a direct integer-literal assignment through one mutable
fixed-width integer parameter, with no extra locals or returned values.
It does not recognize test names or synthesize a contract from the body.
`ensures` uses the typed updated owner; `old(...)` uses its entry value.

At that audit, `NativeMutableGenerated` had nine positive cases, three
false-contract rejections, and five execution/result/loan assertions. It covered
signed/unsigned widths, `old`, conditional abort codes, partial/strict/unspecified
abort behavior, and absent postconditions. The route regression's direct
writer passed natively; its reborrow case remained an explicit rejection.
Failed generation rolls back all native artifacts. These counts describe the
completed batch, not the newer focused extension documented above.

Extra reference locals, reborrows, owner-returning callee summaries, and generic
boundary transport remain unsupported. Neither `clear` nor `drain` is promoted.

Generated owner contracts are translated independently of the assignment body.
The native VC proves the authored predicates; the boundary transports those
facts to the source contract and performs loan export only in the agreement
layer. Dependency guards check both the generated computation and its native
postcondition for encoded values, codecs, frames, and boundary finalization.
Generation uses the existing verification cap synchronously and rejects any
proof-stage error before publishing a native source theorem.

The earlier explicit boundary, mutation/invariant, scalar/nested-loop, and
early-return regressions remain passing. No existing budget or baseline was
relaxed. This is prerequisite implementation, not a completed port of a failing
Check file. Next, compose dereference/checked replacement evaluation with owner
updates, then reference-local/reborrow handling and owner-returning summaries.

Next priorities are native mutable vector/reference loop state (`clear` and
`drain`), reference/storage effects and composition, generic aggregates, and
remaining control/call shapes. Continue existing native migrations before new
v0 ports. Fix diagnostic mismatches by reviewing intended behavior, never by
accepting unsupported positive proofs as expected failures. Performance remains
a completion gate, not a later optimization task.

## Test convention and completion rules

A Check file is LeanerLang source, with contracts, `verify` commands and its
execution/diagnostic assertions. Discover every `Check/**/*.lean`; run each in
its own Lean process under the unchanged 50M per-command verification cap.
Compare all diagnostics to the adjacent `.exp`; absence of `.exp` means empty
output. Expected failures belong in negative cases, not in a baseline that hides
a missing positive proof. `UB=1` deliberately updates baselines and requires
review; ordinary builds must not rewrite them.

Run the Check driver from `leaner-e2e-tests` with
`LEANER_E2E_SUITE=check lake test`. Keep assertion-style IR, Move and Rust tests
in their owning packages. Keep native cost regressions and the separate
`LeanerLang/Tests/Performance` gate; do not raise caps to count a port as done.
Use explicit `#leaner_require_native` assertions for partial migrations and
`#leaner_require_native_all` for completed verification fixtures, in addition to
the automatic audit performed by `verify`.

T0 (Check driver) and T3 (moving hand fixtures into Check) are implemented.
T1 (v0 parity with native verification) is incomplete as recorded above.
T2 (elaborating/verifying frontend-produced LeanerLang) has not started.
Do not run or extend the deprecated packages: their tests are reference material.
No Move Prover differential is planned. Source verification is not a
compiler-correctness theorem for emitted bytecode.

## Audit evidence and maintenance

The per-file tables and completed-batch gates use the 2026-09-08 generated-owner
audit. They have not been rerun for the unfinished changes listed above. Local logs:

- Check summary: `/tmp/dev3-native-owner-check-audit1.log`.
- Per-file results: `/tmp/dev3-native-owner-audit.qHGzPJ/`.
- Native units/executions: `/tmp/dev3-native-owner-unit-gate1.log`.
- Source generation: `/tmp/dev3-native-owner-generator-build10.log`.
- IR: `/tmp/dev3-native-owner-ir-gate1.log`; Move/Rust:
  `/tmp/dev3-native-owner-{move,rust}-gate1.log`.

Older failed boundary builds were superseded by the completed batch, and the
checked-expression square failure by the focused rerun above. The six full-IR
failures and 42 Check failures remain outstanding; no full audit has been run
for the checked-expression changes.

These local logs are ephemeral; the test sources and checked-in baselines are
the reproducible evidence. After each batch, update the date, counts, affected
rows, gate results, and latest-batch paragraph **in place**. Record commits
separately. Put detailed chronology in
[the history archive](test-organization-history.md), not another competing
status table. Preserve original v0 case mappings and semantic decisions there.
