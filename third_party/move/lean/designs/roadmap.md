# Cross-design roadmap

Status: priority ordering agreed 2026-08-27, progress noted 2026-09-03.
This document orders the work across the current designs; each linked design stays authoritative for its
own scope, gates, and deferred-work register. Update the ordering here when a
design's status section changes.

The design documents inside `move/`, `move-model/`, and `transpiler/` describe
the older source-specific path. Their open obligations are not scheduled from
those documents; where they still matter (intrinsic semantics, verification,
compiler-correctness claims) they re-emerge inside the designs below.

## Priorities

1. **Authoritative shared checking** —
   [`lir-design.md`](lir-design.md) Phases 1–2.
   Largely delivered 2026-08-27: `validate` is now a single-run pipeline with
   authoritative name resolution, unification-based typing of executable and
   specification bodies (elided generic instantiations solved with binder
   abilities discharged), and the initialization/borrow analyses run once at
   validate with certificates carried by `ValidatedUnit` (initialization
   failures are validate errors; borrow rejections are recorded and replayed
   by preparation pending loan-death precision). Remaining: dependency
   interfaces before cross-unit signatures or bodies participate in typing,
   interpretation, or verification; the checked trait-selection evidence
   table (the shared blocker also named by
   [`elaboration-design.md`](elaboration-design.md) and
   [`rust-mir-design.md`](rust-mir-design.md)); the certification pass that
   turns `prepare*` into pure mode filters; and the remaining borrow
   checking: general non-lexical loan death, dependency/unknown aggregate
   alias propagation, authoritative nominal variance, and use-site region
   solving.

2. **Verification spine on LIR** —
   [`elaboration-design.md`](elaboration-design.md) M2 through M5, as
   reframed after the 2026-08-27 deprecation of the old packages (M3's
   shadow/differential role passed to the MonoVM harness; M6/M7 collapsed
   into the wholesale deprecation). Finish M2 (interpreter over the
   executable corpus plus the owed metatheory: determinism, completeness up
   to fuel, preservation/no-stuck), then M4 contracts and calculated WP so
   `verify f` reads LIR, and M5 borrow/state/loop/modular verification.
   Decided 2026-08-27 and landed 2026-08-28: references in validated LIR
   are prophecy-based ownership per
   [`prophetic-references.md`](prophetic-references.md), run uniformly by
   the interpreter, the big-step relation, and the verifier; its
   milestones P1–P5 are complete (they replaced the M5 reference bullets
   and the former concrete-to-prophecy refinement obligation), including
   generated reference contracts and the `#leaner_verify` command.
   M4 gate met 2026-08-29: calculation rules cover every expression kind,
   generated contracts read the Move abort discipline (codes and the two
   abort pragmas), obligations carry authored clause ranges and failures
   report at them, and the first reference-stack verification tests
   (`AbortDirections`) prove from LIR with a wrong-body negative baseline.
   Remaining spine work continues under M2's owed metatheory and M5's
   state/loop/modular items.

   The verification half is now designed and tracked in
   [`certifying-execution.md`](certifying-execution.md): the frame-free
   row route, which replaced the frame route retired 2026-09-02 (the
   superseded design is [`historical/verification-v2.md`](historical/verification-v2.md)).
   It is measured by the check ledger of
   [`test-organization.md`](test-organization.md). State 2026-09-03:
   every construct the ported checks exposed is on a generated route —
   references, modular calls, storage brackets, takes and whole writes,
   loops, Rust modular arithmetic, struct returns, generic calls, and
   recursion (the big-step rules are open over a callee oracle and a
   recursive body is the least fixed point of its open body); T1
   continues with the 19 unported v0 verification files, then `Negative/`
   and `Language/`; T2 (the produced LeanerLang verifies) is not started.

3. **MonoVM differential harness** —
   [`monovm-link-design.md`](monovm-link-design.md).
   Design-complete; M0–M3 and M4a implemented 2026-08-27 (adapter staticlib
   with the versioned C ABI and panic containment, BCS marshalling through the
   transaction executor's public call helpers, the Lake link with the
   `lakefile.toml` to `lakefile.lean` migration, the three-way differential
   driver over directive fixtures with recorded `.exp` results and verdict
   tests, and composite values — vectors, nested vectors, addresses, signed
   integers). Remaining: resources and events (M5), and mutable-reference
   out-parameters, which move to M5 with the rest of the reference story.

   The stdlib-consumer half (M4b) is partly delivered: `vector::empty` and
   `vector::length` lower to LIR operations, and a stdlib fixture compares
   clean across all three engines. The `prepareExecution` cost that previously
   blocked it is fixed by the validation rework. What remains is priority 1
   work rather than harness work: LIR operations for the length-changing
   vector natives, value-borrow to place-borrow normalization for element
   borrows, and keeping specification-only locals out of execution
   preparation, which currently blocks every spec-carrying stdlib module.

4. **Rust frontend milestones** —
   [`rust-mir-design.md`](rust-mir-design.md) M0 through M7.
   Close the M0 gate, then the mapper completion and Charon differential
   tests (M1), prebuilt exporter/sysroot distribution (M1.5), contracts and
   `verify` for Rust (M2), regions and prophecy references (M3), drops and
   cleanup (M4), the unsafe profile (M5), library models (M6), and alignment
   proofs (M7).

5. **Surface and backend completion** —
   [`leaner-lang.md`](leaner-lang.md) plus
   [`lir-design.md`](lir-design.md) Phases 4, 6, and 7.
   Implement the LeanerLang forms that are currently stated target language
   (`core.*`, `spec.*`, extension and dependency forms); retire the NSIR
   bridge; corpus-wide semantic alpha-equivalence and source maps; then
   Phase 7: derive NSIR and the verification IR from validated LIR and delete
   the trivial-contract/empty-loop-spec synthesis.

## Out-of-band items

- **Upstream rustc ask, filed.** The two Rustc Public API gaps blocking the
  Rust M0 gate (`FnDef` exposes no predicates; no const-fn binder type query)
  are filed as [rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892).
  Nothing more can be scheduled against it here: rustc turnaround runs on
  release cycles, and this remains the only item whose clock runs on someone
  else's schedule. Until it lands, the generic trait RawUnit fixture cannot
  satisfy the M0 gate, and the answer is still an upstream query rather than
  a private `rustc_middle` read in the production mapper.
- **Security boundary, keep tracked.** Lean-source package discovery executes
  arbitrary host code at compile time; see the TODO in
  [`lir-design.md`](lir-design.md#todo-secure-lean-source-discovery-and-elaboration).
  Not blocking research work, but it must be fixed before Lean-source
  discovery is a production compiler feature.

## Historical designs

[`historical/`](historical/) holds designs that were executed or
superseded. They keep their rationale and measurements, are linked from
the design that replaced them, and are not updated:
[`verification-v2.md`](historical/verification-v2.md) (moved 2026-09-03,
replaced by `certifying-execution.md`).

## Deliberately not scheduled

Phase 0 of [`lir-design.md`](lir-design.md) is written as a multi-party
approval/freeze phase. With one owner on both sides of every boundary there
is nothing to freeze: the "no semantic construct is unclassified" gate is
mechanized by the exhaustive feature classification in
[`../leaner-ir/LeanerIR/Validation/Capability.lean`](../leaner-ir/LeanerIR/Validation/Capability.lean),
and the RawUnit JSON v1 "approve and publish" register row becomes real work
only when a consumer outside this repository depends on the format.

[`unsafe-pointers.md`](unsafe-pointers.md) is a proposal (2026-08-28) for
the Rust unsafe profile over the prophetic model; it is scheduled through
the Rust M5 milestone, not on its own.
