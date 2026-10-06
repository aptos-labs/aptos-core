# Handoff

Status: 2026-10-06, branch `wrwg/lean4` (worktree `dev3`); PAUSED at the user’s request; validated fixes are in the working tree.

## Where the state is

- [`roadmap.md`](roadmap.md): project status and the test ledger.
- [`state-labels.md`](state-labels.md), "Milestones": S1, S2, S3a, S3b implemented; S4 open.
- [`prover-test-problems.md`](prover-test-problems.md): the Prover-test registry.
  The latest full run discovers 437 files: 119 have no diagnostics and 4
  only warnings after the scoped generic-caller fix. All 437 full-run baselines
  matched, and the follow-up passes its ordinary baseline check; intentional negatives
  mean baseline matches are not proof/parity counts.
- [`verification-benchmarks.md`](verification-benchmarks.md): the benchmark.

## Next, in order

Current user goal: address most Move Prover registry gaps and approach MVP
parity. All four Leaner suites passed before starting this work. A fresh
437-file registry run and diagnostic refresh are complete; see the registry’s
Summary. Field updates, abort codes, resource certificates, and several proof
companions are implemented. The latest batch fixes prepared behavioral facts and the count_all companion;
resume only when requested.
Broad suites run at batch boundaries.

1. AMM/calculator valid targets now verify and all four full suites pass.
   Remaining performance work includes fast rejection of AMM’s intentionally
   non-compliant constructor (still 1.5G heartbeats), calculator `process`
   (1.16G), and ordered-map performance: all
   29 targets now verify with zero errors, at 4.126G total heartbeats. The
   largest remaining target is `test_verify_iter_walk_mut_symbolic` (943M),
   followed by drain (439M). A restricted `grind only` companion experiment
   did not close the walk proof and was discarded. Other failed benchmark
   work includes `pool_u64::buy_in`/`deduct_shares`, then nested invocation
   facts in `behavior::add_two`. `capability` verifies completely.
2. A registry entry by test count: V7 intrinsic maps (12), C8 `update` of a spec
   variable (9), V16 natives (7), the `update_field` operation (V1, 11 messages).
3. S4: attribute the remaining `state_labels/` failures. Invocation labels now
   translate in `aborts_if_at_state_label::caller`, but its proof exceeds the
   runner's 25k heartbeat budget. Profile that proof before raising its budget.
   `unmodified_memory_at_label::swap` still needs a decision about reads of
   removed resource contents (see the state-label design).

## This increment

- Checkpoint requested by user: suspend after the current guard validation and
  report refresh; do not begin another fix. This is the validated commit checkpoint.
- Prepared-behavior optimization: manual residual proofs try existing call facts
  before behavioral contract derivation, restoring the goal and using the original
  fallback on failure. Each attempt has a 2M raw-heartbeat cap inside the unchanged
  target budget. The final guard requires a literal closure and permits at most
  two failed attempts per target; successful attempts do not consume this allowance.
  One failed attempt was too restrictive for the intended negative fixture.
- Core and native builds, both cost gates, cross-resource 6/6 at the official 25k
  budget, and behavior_manual_facts at 25k/default all pass. The fixture has three
  positives and one wrong-value negative; the latter retains its normal rejection.
  Logs: `/tmp/behavior-guard-two-{ir-build,move-build,composition,denote,cross-check,
  fixture-25k,fixture-default}.log`.
- Installed `bp_pure_callee.proof.lean`: count_all instantiates the established
  fold equation at 0, i, and i+1, then uses omega. Official scoped baseline update
  and ordinary check pass; only remove_all_found's existing timeout and warnings
  remain. Evidence: `/tmp/behavior-guard-bp-pure-update.log` and
  `/tmp/behavior-guard-two-pure-check.log`.
- Cross-resource preparation drops from 28.437M to 20.974M raw heartbeats using
  existing call facts. Its short companion closes all six targets at 25k;
  higher-order callers retain the original fallback.
- All four full Leaner suites passed immediately before the small final guard:
  `/tmp/registry-prepared-behavior-<package>-{build,test}.log`. That full registry
  refresh passed all 437 baselines, with 119 clean, 4 warning-only, 50 preverification,
  232 verification, and 32 other diagnostic files. Baseline matches include
  intentional negatives and are not proof-success counts.
- Final guard validation completed successfully; no validation processes remain.
  Fresh data: `/tmp/leaner-benchmark-behavior-guard.json`; generated main-relative
  report: `../local_benchmark.html`. Benchmark is 28/32, with no newly failed or
  missing targets relative to update-normalization. Framework ordered_map is
  29/29 at 4,145,490,348 total heartbeats; calculator 8/8 at 1,072,283,339;
  AMM’s six positives pass and its intentional negative remains rejected,
  2,862,902,197 total. Calculator process is 988,943,602 versus 988,919,167
  before this batch; the unguarded regression is removed. AMM constructors
  retain about 1.4M extra heartbeats each, bounded instead of repeated per leaf.
- Full 437-file registry refresh passed; audit has no diagnostic differences
  from the pre-refresh snapshot (which includes the scoped count_all fix).
  Counts remain 119 clean / 4 warning-only / 50 preverification / 232 verification /
  32 other. Logs: `/tmp/behavior-guard-{validation,prover-refresh,registry-audit}.log`.
  Full suites were run before the final small guard; afterward the focused
  fixture checks, both cost gates, benchmark, and full registry refresh passed.
  No additional full-suite run was needed. `git diff --check` passed.
- Next target after resuming: remove_all_found. Diagnostic 100k manual simp_all
  closes, but preparation costs 50.437M raw heartbeats (24 residual leaves 23.096M,
  binds 5.914M, exits 5.210M). Its ordinary std::vector.swap_remove body expands
  bounds/swap/pop cases. Use `/tmp/bp-pure-callee-official-export` captured from
  MVP; standalone CLI export omitted the dependency body and is not comparable.
  Evidence: `/tmp/bp-pure-callee-official-profile.log`.
- Discarded trials: profiling found behavior derivation 7.849M of 28.437M,
  versus state/result rewriting0.171M (`/tmp/behavior-dispatch-profile.log`).
  Sibling proof cache saved only0.178M; pre-split derivation worsened cost and
  disturbed proof contexts; 500k bare simp timed out; skipping behavior for all
  manual proofs helped the direct caller but broke higher-order callers.
  Current bounded PREPARED attempt fixes that distinction; 1.5M sub-budget
  narrowly missed acceptance, 2M succeeds without raising the25k target limit.
- `simple7` remains a semantic observation gap, now V27 in the registry. No
  alias rewrite installed: current-owner tracking must account for reborrows,
  reassignment, branch joins, and old/saved-label observations.

- Read-only diagnosis during the live validation: `specs_in_fun_ref::simple7`
  is not a timeout. The true-25k residual is `False` because `let a := x`
  compiles to `Term.take x`, emptying x's slot; `assertionConditions` then
  requires that original slot to be defined for `assert x == y`. Reproduction:
  `/tmp/specs-in-fun-ref-current.{lean,log}` and
  `/tmp/specs-in-fun-ref-residual.{lean,log}`. A sound fix needs current reference
  ownership/alias tracking at the observation site. Substituting the entry
  value or the reference's final prophecy would be wrong when later writes occur.
  No alias-semantics changes installed.
- Discarded performance experiment after full validation: batching consecutive
  syntactic forall introductions in `Close.splitOnce` reduces cross-resource
  behavior splits 79 → 12 but preparation only 28.437M → 28.383M raw heartbeats;
  it still times out at true 25k. Both cost gates pass, but this is not the
  bottleneck. Source restored from `/tmp/batched-binders-before-Close.lean`.
  Restore build `/tmp/batched-binders-restore-build.log` PASS (session 9046,
  exit 0); source matches the pre-experiment snapshot. No native Move rebuild
  is needed because the original core is restored. Experiment and baseline logs:
  `/tmp/batched-binders-cross-resource-{25k,profile}.log`,
  `/tmp/cross-resource-binder-baseline.{lean,log}`, cost logs
  `/tmp/batched-binders-cost-{composition,denote}.log`.
  One premature scoped anchor check ran while the experimental build had
  temporarily removed Close.olean; its missing-object error is infrastructure,
  not a baseline regression, and its baseline was not changed. Repeated after
  restoration: ordinary anchor check PASS (`/tmp/update-normalization-anchor-check.log`).
  Full suites and full registry above completed BEFORE the discarded experiment.

- Latest installed follow-up: Contract.updateNominalField is now irreducible
  during definitional reduction, with kernel-proved lir_denote_norm equations
  for optional reads, nominal inputs, and unit. This prevents irrelevant
  runtime-constructor splits on typed resource reads. Snapshot:
  `/tmp/update-normalization-before-Contract.lean`. Core and native builds pass:
  `/tmp/update-normalization-{ir,move}-build.log`. New companion
  `state_labels/spec_fun_cross_resource.proof.lean` proves both update-labelled
  callers at true 25k using a codec round trip and explicit resource lookup
  cases. Official registry update and ordinary check PASS:
  `/tmp/update-normalization-registry-{update,check}.log`; baseline now only
  test_behavior_cross_resource's existing 25k timeout (5/6 positives prove).
  Both cost gates pass (`/tmp/update-normalization-cost-*.log`). Existing
  field_update_labels default baseline and both new SourceVerify fixtures pass
  (`/tmp/update-normalization-fixtures-default.log`). New fixtures also pass
  25k (`/tmp/update-normalization-fixtures-25k.log`). field_update_labels' negative
  timed out at 25k both BEFORE and AFTER this change; do not treat that stricter
  scratch cap as its established baseline. Its positive caller's closer drops
  21.5M → 12.8M raw heartbeats (`/tmp/field-update-{before-normalization,current}.log`).
- COMPLETED `/tmp/update-normalization-validation.py`, session 52911, exit 0:
  fresh benchmark/main HTML (28/32, no newly failed/missing targets), all four
  full suites, and 437-file registry refresh PASS. Framework ordered_map 29/29,
  4,145,409,644 total heartbeats; calculator 8/8, 1,072,258,030; AMM six valid
  plus intended negative, 2,860,116,991. Logs `/tmp/update-normalization-*`,
  package logs `/tmp/registry-update-normalization-<package>-{build,test}.log`.
  Audit: 118 clean/4 warnings/50 preverify/233 verification/32 other.
  Newly proves anchor_mutation_pre_state::valid_derived_claim; its negative
  still fails. Aliasing adds one clause diagnostic before the same timeout;
  no newly failed targets. Audit `/tmp/update-normalization-registry-audit.{json,log}`.
  Production freeze is over. No commits made.
- The previous closure-frames controller COMPLETED successfully: fresh benchmark
  28/32, no new failed/missing targets, main HTML, full437 refresh. Audit
  `/tmp/closure-frames-registry-audit.{json,log}` has no baseline changes vs its
  pre-refresh snapshot:118 clean/4 warnings/50 preverify/233 verify/32 other.
  It deliberately did not repeat full suites; the current controller does.
- Remaining cross-resource caller: manual residual proof simp_all closes at a
  diagnostic100k budget, but closer itself costs28.5M raw HB (cap25M), then the
  script1.6M. Diagnostic profile `/tmp/cross-resource-both-native.log`.
  Making StateOf or RuntimeValue.asBool locally irreducible and adding a bool-read
  normalization lemma did not fix it; scratch-only, not installed. Optional-read
  update lemmas only helped once updateNominalField was irreducible, now installed.
  Scratch proof development `/tmp/cross-resource-{entry-native,both-proofs}.lean`;
  installed companion has no diagnostic budget changes or local attributes.
  Next independent gap worth diagnosing: functional/specs_in_fun_ref::simple7
  still fails assertion x == y after moving x's reference to a and writing a;
  no other clause failures are reported. Inspect moved-reference specification reads.


- Current follow-up: closure-frame widening fixed in Close. The verified-target
  frame implication did not unfold `Weave.compose`, although its precondition
  proof did. An extracted implication fails without that normalization and
  passes with it using the existing solver. Final Close diff versus
  `/tmp/cross-resource-frame-before-Close.lean` is only adding Weave.compose to
  that simp list. The extra grind and diagnostic logging experiments are removed.
  Core/native builds: `/tmp/cross-resource-frame-compose-ir-build.log`,
  `/tmp/cross-resource-compose-move-build.log`. Both cost gates pass unchanged:
  `/tmp/cross-resource-compose-cost-{composition,denote}.log`.
  SourceVerify/closure_frame_widening proves six positives at true 25k, including
  a captured argument, and rejects the callback writing an unauthorized captured
  address. Baseline update/check pass (`/tmp/closure-frame-fixtures-*.log`).
  Larger cross-resource residuals lose both FramedAt obligations, but their memory
  and label obligations remain. The optional-read/updateField lemma experiment
  did not change those residuals and is scratch-only, not installed.
- Completed controller `/tmp/closure-frames-validation.py`, session 27220, PID 676365:
  fresh benchmark `/tmp/leaner-benchmark-closure-frames.json`, main-relative HTML,
  target comparison against returned-freeze-final, then full 437-file baseline
  refresh. Logs `/tmp/closure-frames-{validation,benchmark,report,prover-refresh}.log`.
  No four-package full-suite repeat for this small follow-up; those passed just
  before it. Production frozen until controller completes. Audit full registry
  after completion, especially previously failed sample target lists and any
  replacement frontend errors. Then continue resource/label proof automation.


- Latest verified checkpoint: `/tmp/returned-freeze-final-validation.py` exited
  successfully. All four full suites and the 437-file refresh PASS. Fresh full
  benchmark `/tmp/leaner-benchmark-returned-freeze-final.json` has 28/32 with no
  newly failed or missing targets; main-relative HTML generated before suites.
  Framework ordered_map: 29/29, 4,145,487,311 heartbeats; calculator: 8/8,
  1,072,225,189; AMM: 2,860,062,743, six positives plus intended negative.
  Audit `/tmp/returned-freeze-final-registry-audit.{json,log}`: 118 clean,
  4 warnings, 51 preverification, 232 verification diagnostics, 32 other.
  No previously proved target regressed. Earlier E2E/type regressions are fixed.
- Subsequent C18 frontend follow-up is now installed: logical value-typed
  borrowGlobal normalizes to global; reference-typed runtime borrows retain
  semantics. Snapshot `/tmp/projected-global-before-Encode.lean`; native build
  `/tmp/projected-global-move-build.log` passes. SourceVerify/projected_global_reads
  covers four positives at true 25k, opaque caller labels without callee points,
  and one intended negative. Updated and ordinary checks pass:
  `/tmp/projected-global-fixtures-{update,check}.log`.
  Scoped spec_fun_cross_resource registry update/check passes; increment,
  flip_flag, apply prove, all three larger callers time out at 25k.
  `/tmp/projected-global-registry-{update,check}.log`. Counts after scoped update:
  118/4/50/233/32. The full benchmark/suites precede this small follow-up;
  refresh them at the next batch boundary, not after every small change.
  Profile test `/tmp/projected-global-profile-test.log` passes.
  Next: improve larger caller automation (diagnostic residuals below), then other
  registry gaps. Mutable-selector C18 still requires V20, not borrow peeling alone.


- Current batch: C16's implicit reference freezing is normalized in the Move
  frontend. Call results keep the declared instantiated type, then freeze the
  required tuple components; computed freeze operands are evaluated once into
  locals. A subsequent `bp_forwarding` rendering error exposed quantifier-domain
  predeclaration seeing nested locals outside their lexical scope. That is fixed
  in Lower. Print and Close experiments are reverted; this batch changes Encode
  and Lower, plus two SourceVerify fixtures. Seven positive regression targets
  prove at 25k; exactly two intended negatives remain. New fixture baseline
  update and non-update checks pass. Frontend tests and both cost gates pass.
  Both C16 registry files reach verification; two bp_forwarding positives prove,
  and four other positive loop targets still exceed 25k. See C16 in the registry.
  Snapshots: `/tmp/returned-freeze-before-{Encode,Lower}.lean`.
  Native build: `/tmp/returned-freeze-move-build6.log`.
  First validation finished: fresh benchmark `/tmp/leaner-benchmark-returned-freeze.json`
  retains 28/32 with no target regressions; main HTML generated before suites.
  IR, Move, and Rust full suites passed. E2E exposed a real smart_table rendering
  fixed-point regression: runtime freeze materialization had also affected logical
  operands. The encoder now keeps logical freezes unchanged. This also restores
  two registry vector_hofs files whose type errors had masked their proof targets.
  Native correction build: `/tmp/returned-freeze-move-build8.log`. Seven positive
  SourceVerify targets now pass; two intended negatives remain, with updated and
  checked baselines (`/tmp/returned-freeze-fixtures-{update2,check3}.log`).
  Full 437 registry refresh passed; scoped corrections for vector_hofs_for_each
  and vector_hofs_mut_receiver are refreshed. Audit:
  `/tmp/returned-freeze-registry-audit.{json,log}` (JSON exists; regenerate log).
  Counts using the established diagnostic categories: 118 clean, 4 warnings,
  51 preverification rejections, 232 verification diagnostics, 32 other errors.
  These broad categories include six preexisting Lean elaboration errors under
  verification diagnostics; removed proof names require inspecting replacement
  diagnostics, not treating them automatically as progress.
  All focused Move rendering baselines PASS (session 93339 exited zero):
  `/tmp/returned-freeze-corrected-format2.log` (first attempt lacked CLI env).
  Completed final controller `/tmp/returned-freeze-final-validation.py`, session 79804,
  PID 653603: corrected fresh benchmark/main HTML, all four full suites, then
  437-file registry refresh. Logs `/tmp/returned-freeze-final-*`; package logs
  `/tmp/registry-returned-freeze-final-<package>-{build,test}.log`.
  Controller completed; results are summarized at the top.

  The arithmetic-only normalization discharger experiment had no useful gain;
  it was reverted (`/tmp/context-discharge-before-Close.lean`).
  Budget-unit correction: CLI `--heartbeats` / SourceVerify `heartbeats` are
  thousands of maxHeartbeats units. Use 25, not 25000, to match the registry.
  Raw `-Dleaner.verifyHeartbeats=25000` is correct. The focused fixture driver
  was corrected and passes at the true 25k cap:
  `/tmp/returned-freeze-fixtures-true25k.log` (seven positives, two negatives).
  Earlier cross-resource scratch claims used the oversized CLI argument;
  the combined test_behavior_cross_resource times out at true 25k, though the
  primitive functions prove. Its frontend normalization remains promising.
  Ready C18 regression: `/tmp/projected_global_regression.move`. Unmodified
  frontend reproduces `$increment: borrow result is not a reference type`
  (`/tmp/projected-global-before.log`). Scratch export changes only value-typed
  borrow_global to global, preserving reference-typed runtime borrows; verification
  at CLI --heartbeats 25 leaves exactly wrong's two intended diagnostics
  (`/tmp/projected-global-regression-25k.log`). Install after current validation:
  addExpr in logical mode can normalize a non-reference borrowGlobal node to
  `.global none` with the same type, location, instantiations, operands and surface.
  Add focused SourceVerify fixture and regenerate its expectation using 25.
  Larger caller residuals at diagnostic-only raw budget 100000 are in
  `/tmp/cross-resource-residual-diagnostic.log`: frame widening for literal
  increment/flip closures and impossible missing-resource/updateField branches.
  No larger acceptance budget was installed. V20 needs a genuine mutable-input/
  writeback invocation model; peeling projected borrows alone is insufficient.
  Next-gap scratch probes (not installed): C18's spec-function value-typed
  `borrow_global` should become a logical global read; replacing it in the
  cross-resource export lets increment/flip_flag/apply/test_behavior_cross_resource
  prove, leaving two labelled wrapper proofs unproved without timeouts
  (`/tmp/spec-fun-cross-resource-probe2.log`). Mutable selector BPs need both
  peeling a value-typed borrow and expanding implicit pre/post arguments; even
  then they hit V20's unsupported mutable-parameter behavioral predicates
  (`/tmp/bp-mut-ref-selector-probe2.log`). Preserve caller-side label semantics.

- Current performance batch: kernel-proved `Flow.pure_shortCircuit_and/or`
  combine pure boolean guards only when both branches preserve the same locals.
  This avoids duplicate continuations. A bounded arithmetic-only omega attempt
  precedes the full-context solver in the cheap bound decider. The new
  `ArithmeticContext` test checks the fast path and fallback; `ControlForms`
  covers effectful short-circuiting. Registry `constant_product` now proves at
  25k with its installed companion; its closer is 15.1M heartbeats / 17 leaves.
  Logs: `/tmp/short-circuit-ir-build2.log`, `/tmp/short-circuit-move-build2.log`,
  `/tmp/short-circuit-arithmetic-test.log`, `/tmp/short-circuit-control-test.log`,
  `/tmp/short-circuit-registry-amm.log`. The remaining fee/swap/constructor
  obligations still need work at 25k.
  The first benchmark/report completed with 28/32 and no target regressions:
  `/tmp/leaner-benchmark-short-circuit.json`. AMM is 2.55G (previous 2.87G), with
  six valid targets and the intended negative retained. Move/Rust suites passed;
  IR's cost gate caught proof-term growth on simple arithmetic, and E2E found
  `Scalars/Order::ordered3`'s authored proof depended on the old branch split.
  The fast path now requires memory-dependent hypotheses; its prototype passes
  the cost gate. `ordered3` now introduces both comparison premises explicitly
  and passes. The 437-file refresh passed, with no new failed functions:
  `/tmp/short-circuit-registry-audit.log` (118 clean / 3 warnings / 53 preverify /
  231 verification / 32 other). Two changed diagnostics: shorter aborts_if
  residuals and bug_15044 now fails normally instead of timing out.
  The corrected final benchmark completed: 28/32, no newly failed targets,
  `/tmp/leaner-benchmark-short-circuit-final.json`. HTML was generated against
  main before the full suites. AMM is 2.86G versus 2.87G; constant_product is
  16.1M versus 26.8M (-39.8%). The earlier non-compliant-pricing improvement
  did not survive the memory-context restriction needed by the cost gate.
  Both corrected cost gates pass. `/tmp/short-circuit-final-validation.py`
  completed successfully: all four full suites pass, as does the 437-file
  registry refresh. The final audit found no changed diagnostics (118 clean,
  3 warnings, 53 preverify, 231 verification, 32 other). Logs use
  `/tmp/short-circuit-final-*` and `/tmp/registry-short-circuit-final-*`.
  Vector-fold probes show
  base/exit cases close after context normalization and recursive-spec
  unfolding, but loop-step preparation still exceeds 25k. Even an unconditional
  kernel-proved concrete prefix formula times out; repeated normalization and
  conditional arithmetic remain costly (`/tmp/fold-unconditional-debug.log`).
  No fold experiment is installed. The subsequent `specialize_generic_caller`
  companion fixes `use_concrete` at 25k: inlining omits the callee's proof block,
  so instantiate its proved `count_is_len` lemma at u64 for entry/exit/back-edge
  obligations. Official scoped refresh and non-update baseline check pass
  (`/tmp/specialize-generic-caller-registry-{update,check}.log`). Only the
  existing compiler warning remains. Current counts after that follow-up:
  118 clean / 4 warnings / 53 preverify / 230 verification / 32 other.
  The full suites above precede this proof-only change. No broad rerun is needed
  after each companion; keep the user's benchmark-first batch workflow.
  Sound concrete-loop unfolding would need
  to prove actual exit; MVP's bounded-cut `pragma unroll` is not acceptable.


- Current batch: explicit stored-field write frames are implemented across
  export, validation, printing, contracts, and kernel-checked encoding lemmas.
  Addressed/wildcard frames, generic resources, opaque returns, and captured
  owners pass focused checks at 25k. The new SourceVerify fixture has exactly
  three intended failures; the Check error fixture has four. An invalid
  frame conversion now fails normally instead of exhausting recursion.
  Resource-invariant dispatch is computed on newly derived facts and written
  values, avoiding a whole-context simplification. `stored_fun_values` now
  proves 33/42 targets (33/35 MVP-positive) with no heartbeat timeouts. A
  kernel-checked companion transports invariants through addressed frames.
  Seven failures are intended; one wildcard call lacks an invariant-preservation
  guarantee, and `create_modifier_valid` violates its implicit empty write
  frame. See V26 in the registry. Dependency interfaces reject explicit field
  frames until they can carry the owning contracts; they cannot silently
  substitute the default empty frame.
  Snapshots before the batch: `/tmp/stored-frames-before`.
  Core/native builds: `/tmp/stored-frames-ir-build7.log`,
  `/tmp/stored-frames-move-build5.log`. Focused validation:
  `/tmp/stored-frames-positive7.log`, `/tmp/stored-frames-fixtures-check.log`,
  `/tmp/stored-frames-prover-followup.log`. All four full suites passed,
  including 122 Check fixtures and both cost gates: logs
  `/tmp/registry-stored-frames-<package>-{build,test}.log`. The 437-file registry
  refresh passed (`/tmp/stored-frames-prover-refresh.log`): 118 clean, 3 warnings,
  54 pre-verification rejections, 230 verification diagnostics, 32 other errors.
  Move's full suite passed again after the interface guard. The first full
  benchmark exposed two AMM proof-script regressions hidden by unchanged sample
  status; both scripts now tolerate earlier invariant normalization. The final
  full rerun `/tmp/leaner-benchmark-stored-frames-final.json` passed the target-level
  regression comparison (`/tmp/stored-frames-final-status.log`), retaining 28/32
  passing samples and all six valid AMM targets. `local_benchmark.html` is
  regenerated against main, without intermediate runs. Framework ordered_map
  is 29/29 at 4.142G heartbeats; calculator is 8/8 at 1.071G.
  Scoped follow-up: implicit function-wrapper conversions (C20) now emit ordinary
  packs/projections with instantiated field types; both printer stages quote
  positional frame targets. `SourceVerify/function_wrappers.move` proves all
  eight valid targets at 25k and rejects its invalid pack. Frontend formatting
  and Move profile checks pass. Logs: `/tmp/function-wrappers-fixture-check.log`,
  `/tmp/function-wrappers-frontend-test3.log`, `/tmp/function-wrappers-profile-test.log`.
  Core/native builds: `/tmp/function-wrappers-ir-build2.log`,
  `/tmp/function-wrappers-move-build6.log`. Registry AMM now reaches verification
  (`/tmp/function-wrappers-registry-amm-final.log`), with `create_pool` proved,
  four pricing/swap timeouts and three dependent constructors. The attempted
  arithmetic companion also timed out at 25k; it is only in
  `/tmp/registry-amm-arithmetic-candidate.proof.lean`. This scoped follow-up is
  not covered by the preceding full-suite/benchmark runs. Next: profile AMM
  preparation/proof cost at 25k, then continue the registry; refresh benchmark
  data/HTML before the next broad suites.

- Validated milestone (2026-10-06): fresh full benchmark
  `/tmp/leaner-benchmark-registry-final.json`, HTML against main CI
  `ea4ecc43e7`/run 37250691414, then all four full Leaner suites passed.
  Logs: `/tmp/registry-final-<package>-{build,test}.log`. All 120 Check
  fixtures and both cost gates passed. The 437-file registry refresh passed
  (`/tmp/registry-final-prover-refresh.log`), with counts in the registry.
  The benchmark keeps all 28 previous passing samples; passing aggregate
  heartbeats change +0.16% from the prior abort-code batch. Calculator 8/8
  at 1.07G; framework ordered_map 29/29 at 4.14G. AMM's six valid targets pass.
- Runtime resource lookup now excludes open generic templates; frame
  coherence retains the template lookup. Memory typing and encoding share
  the closed-key lookup. `GenericResources` covers both classes. Constant
  vector indexing resolves the constant as a value rather than a resource.
  Encoded stored-field behavioral facts normalize in a bounded speculative
  attempt that restores context on failure. No global `NTy.encode_function`
  normalization attribute remains.
- Nominal quantifiers containing function fields remain explicitly rejected
  until the MVP field-validity domain is modeled. Unrestricted field-row
  quantification was shown to prove the invalid `other_instantiation` in a
  scratch experiment; that expansion is not applied. Four direct-field
  positives now verify, and generic-aliasing negatives stay rejected.
- Proof companions now prove `nonlinear_arithm::mul5`, all of
  `invariants_with_quant`, and all of `simple_vector_client`, preserving
  the 25k registry budget. Concrete vectors compute their list spines.
  The follow-up `/tmp/registry-for-loops-final.log` confirms all eleven
  `for_loop_invariants` positives and rejects both incorrect cases.
  Its triangular-division rewrite avoids unnecessary absolute-value cases;
  all three sum proofs now fit 25k. The regular benchmark inputs are unchanged.
- In progress after the full suites: generic enum invariants now lower their
  `this` type using the declared type arguments, as struct invariants do.
  Core and Move driver builds pass. `enum_19575` verifies, and the new source
  fixture `generic_enum_invariants.move` proves generic constructors/readers
  and an opaque caller while rejecting invariant/postcondition violations,
  also checked at the registry's 25k budget.
  This scoped follow-up raises the registry's no-diagnostic count to 116;
  the four full-suite logs above precede this lowering change.
  Next diagnosed issue: in-body assertions over live mutable borrows read
  the parent’s prophecy, not the holder’s current value (`bug-17117`, and
  likely `bug_15880`). `/tmp/live-borrow-probe.lean` isolates it; its leaf
  asks `value.val = r_x.val` without a connecting fact. Inspect
  `Verify.assertionConditions`/`overDefined` and the compile loan facts; a
  current-value view must reconstruct parents from active holders without
  changing caller-side state labels or ending loans that are still used.

- Next registry batch is in progress: `aborts_with` is carried through the
  frontend, printer, lowering, and contract generator. Abort-code coverage is
  checked independently of partial abort conditions, matching MVP. Partial
  conditions without codes now have a trivial failure-permission predicate.
  The remaining positive `conditional_abort` exposed arithmetic traps modeled
  as user aborts carrying their operands/results. Move arithmetic now uses a
  `runtime.arithmetic_error` profile throw; explicit aborts retain their codes.
  Both arithmetic and vector runtime errors map to `EXECUTION_FAILURE` in
  contract code clauses. The two MVP abort fixtures now prove all 12 valid
  functions and reject all 15 intentionally invalid ones. The differential
  arithmetic-overflow fixture now matches MonoVM instead of reporting abort 300.
  Fresh benchmark `/tmp/leaner-benchmark-registry-aborts.json` preserves all
  28 passing samples; their combined heartbeats decrease 1.5%, with calculator
  down 13.9%. HTML compares this run against main. All four full suites pass after report generation, including MonoVM and
  differential checks (`/tmp/registry-aborts-leaner-e2e-tests-test5.log`).
  The checks now distinguish arithmetic errors from explicit user aborts;
  three partial-code fixtures now cover their previously omitted abort
  boundaries. Existing negative proofs remain rejected. The full MVP
  refresh matches all 437 baselines, with aggregate diagnostic categories
  unchanged (`/tmp/registry-aborts-prover-refresh.log`).

- Registry follow-up in progress: functional `update_field` now renders,
  parses, lowers, and translates into contracts, including generic and nested
  updates and fields at different enum positions. Partial-variant and intrinsic
  map updates remain explicitly unsupported. Lowering `old` now selects the
  pre-state before descending into its operand; this removes false cycles in
  old-only state-label definitions. A bounded decoded-memory closing attempt
  handles the opaque caller case without ProgramPoint witnesses. The new
  `FieldUpdate` Check fixture passes four proofs; the source fixture
  `field_update_labels` verifies the correct caller and rejects `caller_wrong`.
  Fresh full benchmark `/tmp/leaner-benchmark-registry-field.json`: 28/32
  verified, preserving every previously passing problem; their combined
  heartbeat cost changes by +0.1%. Framework ordered_map keeps all 29 proofs
  at 4,128,189,059 heartbeats; calculator keeps all eight. The HTML was
  regenerated against main before starting the four full Lake suites.
  All four full Lake suites pass, including 119 Check fixtures and both
  performance gates. The full registry refresh matches all 437 baselines;
  aggregate diagnostic counts are unchanged, with V1/V2 updated to the more
  specific remaining failures. Logs: `/tmp/registry-field-*-test.log` and
  `/tmp/registry-field-prover-refresh.log`.

- AMM/calculator follow-up: calculator now verifies in the regular native
  benchmark, including its callers. Its specification captures the old
  continuation before selecting the post-removal label; reading the removed
  resource inside that label selected an absent value in Leaner's partial memory.
  Stored function fields now carry the Move default read-only frame through
  construction, mutation, storage, removal, and invocation. The compiler and
  contract generator share the predicate deciding which declarations require
  these checks. Explicit field `modifies_of` is rejected until supported.
  Behavioral data invariants request the typing/determinism and termination
  assumptions their clauses need. The closer retains arithmetic names used by
  saved continuations, clears unused continuation lets before authored proofs,
  and uses the frame equation when identifying an invocation's final state.
  New positive and negative stored-field fixtures cover these boundaries.
  AMM's companion proves arithmetic pricing laws and valid construction/swap
  obligations; the intentionally non-compliant constructor must remain rejected.
  Full benchmark `/tmp/leaner-benchmark-amm-calculator-complete.json`: **28/32
  verified**; calculator has zero errors, all six valid AMM targets pass, and
  ordered_map retains all 29 passing targets. The other failures remain
  `type_info`, `pool_u64`, and `behavior`. Generated HTML compares this one
  fresh full run against `main`, without intermediate local runs. All four
  package builds and full Lake suites pass, including 118 Check fixtures,
  source verification, MonoVM/differential checks, and both performance gates.
  Existing baselines were not regenerated; the new negative field-frame
  fixture has its own reviewed baseline. Logs: `/tmp/amm-calculator-*-test.log`.

- Latest ordered-map result: **29/29 targets verified, zero errors**. Total
  heartbeats fall from **6,900,704,697 to 4,125,866,249** (40.2%). The driver
  regenerated benchmark data and HTML before the final broad test run.
  All four Lake suites pass, including end-to-end, MonoVM, differential tests,
  and both performance gates, without baseline changes for this increment.
  The first full run caught lost loop-invariant diagnostic markers; matching
  now normalizes only abort-continuation markers and preserves source clauses.
  See the latest entry in `perf-notes.md` for fixes and remaining costs.

- Full benchmark after these fixes: **24/32 verified**, up from 23/32 in
  the fresh baseline; neither map problem crashes. `ordered_map` uses 7.8%
  fewer heartbeats (13.37G → 12.34G), with 16 failing targets instead of 17.
  Other previously verified problems have effectively unchanged heartbeat
  costs. All four package builds/suites and the Python isolation test pass.
- Benchmark fixes: the registry carries the two insertion-position roles now
  used by `simple_map`, preserving its sequence discipline; `InsertionMap`
  checks descending keys and ranks. This unblocks `simple_map` and `pool_u64`
  before verification. The invocation projection scan visits shared terms
  once. Both performance gates pass without updating their baselines.
- `scripts/isolate-target.py` now disables assertion-only functions and omits
  unrelated proof commands. Earlier isolated map runs also verified these
  extra targets; inspect the JSON `targets` array before attributing timings.
  See [`perf-notes.md`](perf-notes.md), "Failed benchmark targets".

- Invocation-defined labels use `Proofs.StateOf`, sharing the successful
  invocation selected by `ResultOf`. A kernel-checked determinism proof
  identifies its post-state, including unnamed resource types. No new axioms.
  Naming a label adds no assumption that the invocation succeeds.
- These state expressions survive substitution of an opaque callee's contract
  at its caller; they do not depend on internal `ProgramPoint` witnesses.
  Call and terminating-run rules expose post-state equations to the closer.
  Literal callable encodings retain a reducible native-type hint so invocation
  projections can recover their typed arguments and results.
- `InvocationStateLabels`: forward definitions, chained invocations, opaque
  callers of `result_of` and `ensures_of`, and function-valued parameters.
  `InvocationStateLabelErrors`: an always-aborting call fails its intended
  `aborts_if false` clause. The Prover's `aborting_result_definition` likewise
  reaches its intended failures; invocation-label unsupported errors are gone.
- `Contract.lean` translates the state-change predicates and resolves their free
  labels recursively, across clause order and chains. A definition does not assume
  its predicate, even under negation or an implication antecedent.
- `Check/Specifications/DefinedStateLabels.lean`: direct changes, generic resources,
  chained labels, local spec bindings, and an opaque caller proving fields and frames.
  `DefinedStateLabelErrors.lean`: false clauses and a removal that can abort.
- `mixed_callee_postcondition` and `bp_inline_derive` now verify. All 30 state-label
  Prover tests and the additional inline test match their reviewed baselines.
- The design records the remaining `unmodified_memory_at_label::swap` gap: Boogie
  retains a removed resource's contents, whereas Leaner's partial memory discards
  them. Its negative `swap_wrong` fails as intended.
- Validation: builds and full tests passed in all four packages, including
  all 113 Check fixtures, source verification, MonoVM, and differential tests.
  Both verification cost gates passed without baseline changes. The full
  end-to-end suite and 31 Prover tests passed without baseline regeneration.

## Open findings not yet in a design

- Benchmark, full run at e4fb2de4cc against 7457d37: 24/32 verified, heartbeats
  +1.3%; `comparator` +18.6% and `string` +11% heartbeats, not yet attributed to a commit.
- The frontend blockers for benchmark problems `amm` and `calculator` are fixed:
  condition attributes preserve values such as `inferred = sathard`, proof steps
  substitute phase-appropriate contract lets, post-only labels retain grouping, and
  free label definitions retain lexical let bindings. Captured function literals
  now translate in specifications and compare with executable closure values.
  Both samples reach proof obligations; their remaining failures are no longer
  unsupported syntax or unknown locals. The AMM companion proves the three pricing
  implementations without changing their Move specifications. The deliberately
  non-compliant pool constructor must still be rejected.
- The three failing `ristretto255` targets now verify using a companion proof:
  normalize option payloads and apply byte-vector codec tightness. No cryptographic
  code, native model, or specification changed.
- The entire `capability` module now verifies. The companion normalizes address
  and parameter transport, proves that writes to one generic resource declaration
  preserve reads of another, and reduces stored-value transport round trips.
  This closes `delegate` and `revoke` as well as both acquisition targets,
  without changing Move code or specifications.
- `ordered_map` investigation found expensive `omega` calls before authored
  proofs: 508M heartbeats on one prepared enumeration leaf, then a timeout on
  the next. Residual deciders now use the existing 20M speculative-attempt
  bound. The companion adds proofs for `ground_enum_123`, duplicate-key
  construction, and front/back key borrowing. Further enumeration, lower-bound,
  and loop failures remain. The native benchmark drops from 12.336G to
  11.043G heartbeats (10.5%); `ground_enum_123` drops from a 1.5G timeout
  to 264M. See `perf-notes.md` for the remaining expensive targets.
- Follow-up: `test_verify_lower_bound_gap_symbolic` now verifies automatically
  at 37.8M native target heartbeats versus 1.078G failed. Normalize enum payloads
  and canonicalize `NTy.encode`/`HList.encode` across generic caller/callee
  families so the contract's encoded index and caller's integer are one term.
  No Move/specification changes or manual gap proof. `GenericEnumPayload`
  covers the generic opaque enum-return case. The whole module rerun drops
  from 11.043G to 8.379G heartbeats (24.1%); other targets still fail (43 errors).
  Benchmark data and HTML are regenerated; see `perf-notes.md`.
- Next most expensive target, `test_verify_remove_or_none`, now verifies with
  its companion proof at 124.9M native target heartbeats, down from the 1.501G
  timeout. Proven constructor/size/membership reductions keep its literal
  three-entry map computational before residual proof search. No core verifier,
  Move source, or specification changes; no broad suites run for this increment.
  The module benchmark confirms 124.0M target heartbeats and 8.379G → 6.901G
  total (17.6% fewer); `test_verify_upsert` also stops failing, with no new
  failures. Six module targets remain failed; data and HTML are updated.
- Prover test `generic_aliasing_all_partitions::never_alias` now exhausts its budget
  (was a clause failure) since S3b.
- From the S3b cleanup review, not done: `singleFieldRead?` (one integer field) and
  `boundWitnesses` (`<`, `≤`, `=` with the binder on one side) are the instances of "a binder
  read only through projections" and "the binder's interval from the bounds machinery".
  The `subst` and `projections` leaf stages run for every leaf of every target; whether
  they explain the `comparator`/`string` heartbeat growth needs a measurement.
- A quantifier over a vector's elements is not instantiated at a read position:
  `requires forall x in v: x < 10; ensures v[0] < 10` (with `len(v) > 0`) fails, by value
  and by reference alike, while `forall i in 0..len(v): v[i] < 10` verifies.
- `leaner_denote_witness` decides the conjuncts reading the binder first, in the order
  `Array.qsort` leaves them; a stable order puts `verify_vector::verify_contains` over its
  budget, so the search depends on an incidental order.

## How to work

- User preference: during benchmark/performance work, run the affected benchmark
  and regenerate its data and HTML **before** broad test suites. Use focused
  checks for small iterations; run the full suites periodically or after a
  substantial batch, not after every small change. Heartbeats are the primary
  performance metric.

- Prover tests, from `third_party/move/move-prover`: `MVP_TEST_FEATURE=lean cargo test`
  with one filter per run; `UPBL=1` regenerates the `.lean_exp` baselines. Read every
  changed baseline; an `-- error:` line is a regression unless intended.
- Suites: `lake build && lake test` in `leaner-ir`, `leaner-move`, `leaner-e2e-tests`.
  Never rebuild `leaner-ir` while a suite runs; iterate in a scratch copy.
- A deliberate cost change: `UB=1 lake env lean LeanerLang/Tests/DenotePerformance.lean`
  after `lake build`; the diff names only the targets meant to move.
- Benchmark: `python3 scripts/leaner-bench.py run --out <fresh.json>`, then
  `python3 scripts/leaner-bench.py report --local <fresh.json> --branch main --html local_benchmark.html`.
  Compare against main, not intermediate local runs.
- A new capability gets a Check fixture and a roadmap or design row. A Check fixture never
  records an unsupported positive proof as an expected failure.
- Debugging a target: `set_option leaner.denoteDebug true` and `verify f by skip` show the
  leaf obligations.
