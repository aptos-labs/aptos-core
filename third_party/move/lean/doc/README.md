# Documentation

Read the design for the layer being changed. Designs hold the model and the
decisions, not status; what is open is in the plan, and state that the source
does not show is in the status directory.

| Directory | Contents |
|---|---|
| [`user/`](user/) | The Verex Book: concise user documentation for VerexSL and the proof language, for expert users; not a replacement for the designs. |
| [`designs/`](designs/) | The current designs (below). |
| [`plan/`](plan/) | Open problems, stated as problems. [`roadmap.md`](plan/roadmap.md): what is open, by track, in priority order; [`data-and-global-invariants.md`](plan/data-and-global-invariants.md) and [`proof-language.md`](plan/proof-language.md): the open problems of those two designs; [`unsafe-pointers.md`](plan/unsafe-pointers.md): an approved, unscheduled proposal. [`transitions.md`](plan/transitions.md) is the only place for history: how the designs came to be what they are, one section per transition. |
| [`status/`](status/) | State the source does not show. [`prover-test-problems.md`](status/prover-test-problems.md): what the Move Prover's tests show under Verex and why; [`assumptions.md`](status/assumptions.md): the hypotheses of the public theorems nothing discharges; [`measurement-debts.md`](status/measurement-debts.md); [`verification-benchmarks.md`](status/verification-benchmarks.md): the benchmark's problems and open decisions; [`framework-verification.md`](status/framework-verification.md): what passes on the Aptos framework packages, and why not. |
| [`historical/`](historical/) | Executed or superseded designs, kept for their rationale, not updated. |
| [`talk/`](talk/) | Talk sources and PDFs. |

## Designs

- [`vir-design.md`](designs/vir-design.md): the unified VIR: ownership, stages, profiles, validation, serialization, frontend and backend contracts.
- [`elaboration-design.md`](designs/elaboration-design.md): runtime model, big-step semantics, interpreter, correctness obligations.
- [`denotation.md`](designs/denotation.md): one `denote` of validated VIR into `Spec` and one agreement theorem against `BigStep`.
- [`native-verification.md`](designs/native-verification.md): verification over native values only: constructs, enforcement, the solver's normal form and splits, specialized contracts.
- [`data-invariants.md`](designs/data-invariants.md): data invariants as part of a value's carrier type: where they are owed and assumed, loans, call boundaries, the stored-invariant layer beside them.
- [`given-parameters.md`](designs/given-parameters.md): canonical abstract parameter sorts, nominal invariant cuts, and explicit transfer to concrete interpretations.
- [`proof-strategy.md`](designs/proof-strategy.md): bounded automation, authored proofs, no SMT; the authored interface.
- [`performance.md`](designs/performance.md): how to measure, build cost, shared continuations, callee summaries, what was learned, open levers, levers measured and rejected.
- [`benchmark.md`](designs/benchmark.md): the nightly verification benchmark: measurements, components, reports, CI, local use.
- [`proof-language.md`](designs/proof-language.md): the proof language: surface, obligation addressing, semantics and lowering, preparation, messages.
- [`source-verification.md`](designs/source-verification.md): `verex-move verify` / `verex-rust verify` / `move prove --lean`, proof files, pragmas, message mapping.
- [`verex-sl.md`](designs/verex-sl.md): the profile-aware Verex source language.
- [`prophetic-references.md`](designs/prophetic-references.md): the prophecy ownership model of references.
- [`unsafe-pointers.md`](designs/unsafe-pointers.md): raw pointers and checked memory beside the prophetic model: typed and byte allocations, `transmute` and unions, minted references as memory loans, what unsafe Rust is allowed, MonoVM's unsafe code mapped.
- [`static-memory.md`](designs/static-memory.md): one typed memory per resource type.
- [`intrinsic-maps.md`](designs/intrinsic-maps.md): the abstract map value of intrinsic map types, roles, validity, typed map lemmas.
- [`ghost-fields.md`](designs/ghost-fields.md): ghost fields and validity slots.
- [`state-labels.md`](designs/state-labels.md): state labels as memory variables.
- [`higher-order-functions.md`](designs/higher-order-functions.md): closures in VIR, runtime, denotation, behavioral predicates, the typed carrier of function types.
- [`lemmas.md`](designs/lemmas.md): `spec lemma`s and `proof` blocks.
- [`numeric-representation.md`](designs/numeric-representation.md): integers as mathematical integers, not the Prover's bit vectors.
- [`relocatable-namespaces.md`](designs/relocatable-namespaces.md): namespaces linked by relocation; verification across modules.
- [`rust-verification.md`](designs/rust-verification.md): the model of Rust source verification: the supported surface, the rejection boundaries per stage, failure contract and panic obligations, references in aggregates, slices, closures and `Fn*` evidence as static dictionaries, library contracts at shapes, crates.
- [`rust-mir-design.md`](designs/rust-mir-design.md): the Rustc Public frontend, Rust profile, source backend, import contract and trust boundary.
- [`monovm-link-design.md`](designs/monovm-link-design.md): MonoVM linked for differential execution.
- [`../verex-rust/rust-exporter/README.md`](../verex-rust/rust-exporter/README.md): exporter scope and commands.

The deprecated packages under `v0/` keep their own design documents
(`v0/move/Move/*.md`, `v0/move-model/MoveModel/README.md`,
`v0/transpiler/*.md`); they describe the old stack and are reference only. The
e2e baseline conventions are in
[`../verex-e2e-tests/README.md`](../verex-e2e-tests/README.md).
