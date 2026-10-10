# Opt-in Loop-Invariant Evidence for Specification Inference

## Status

The first cut-point WP slice is implemented on `wrwg/unroll-diag`. It adds the
opt-in bounded-depth interface, isolated forced unrolling, copied-head mapping,
source-oriented rendering, deliberate degradation, and a focused readability
baseline.

The current slice reports entry-to-`head[k]` relations for one missing loop in
a function. It does **not** yet compute a loop-local transition from an
arbitrary symbolic head, and it reports multiple missing loops as unavailable
instead of attempting to attribute a function-level relation to either one.

## Goal

Specification inference already identifies a useful failure mode: a loop with
no invariant is abstracted by havocking its modified state, and the resulting
function contract is marked `[inferred = sathard]`. The current warning points
at the loop and recommends an invariant, but it gives the caller no facts from
which to derive one.

Add an **optional, diagnostic-only output** to inference which describes:

1. the source-visible state carried by the loop;
2. facts at loop entry;
3. the symbolic effect of one back-edge traversal; and
4. path-conditioned states observed at the first few loop heads.

The output helps a human or agent propose an invariant. It does not synthesize,
insert, or validate one.

This feature belongs to the inference command. It is off by default, does not
change ordinary verification, and does not change the inferred Move source.

## User contract

Add an inference option with an optional bounded depth:

```text
move-prover --inference --loop-invariant-evidence[=N] ...
```

- Omitting the option performs no additional analysis and produces no new
  output.
- Supplying it without `N` uses depth 3.
- `N` counts completed back-edge traversals. The report therefore contains
  heads `0..=N`.
- Reject zero and cap the initial implementation at 8.
- Reject the option unless inference mode is enabled.

Inference source output has three existing forms: stdout, per-module files, and
one unified file. Those outputs must remain valid Move source. Evidence is
therefore rendered as diagnostics through the inference error writer, not mixed
into generated source. The current implementation keeps the structured records
in a `FunctionData` annotation until `SpecInferenceProcessor` renders them.

The flag is a request, not a promise that useful evidence exists. Unsupported
or imprecise loops receive a short reason instead of invented facts.

## Example

For a loop which decrements two parameters together, the current diagnostic
shape is:

```text
warning: WP inferred `sathard` conditions after this loop without an invariant
   = loop-invariant evidence (bounded to 3 back-edges; diagnostic only)
   = source-visible loop-carried state: x, y
   = bounded loop-head facts (for paths reaching each head):
       head[0]: head[0].x == x
                head[0].y == y
       head[1]: x > 0 && y > 0 ==> head[1].x == x - 1
                x > 0 && y > 0 ==> head[1].y == y - 1
       head[2]: x > 1 && y > 1 ==> head[2].x == x - 2
                x > 1 && y > 1 ==> head[2].y == y - 2
       head[3]: x > 2 && y > 2 ==> head[3].x == x - 3
                x > 2 && y > 2 ==> head[3].y == y - 3
   = seek a predicate which includes the entry facts and is preserved by one
     back-edge; bounded observations are not an invariant or a proof
```

`head[k]` is diagnostic notation, not Move syntax. Parameter and retained local
names come from `FunctionTarget`/`FunctionEnv`; internal temporaries which
cannot be mapped to source are omitted and reported in the completeness note.

For a branch in the loop body, retain path conditions instead of merging
incompatible states:

```text
head[2], when choose_left:  x == a + 2, y == b
head[2], when !choose_left: x == a,     y == b + 2
```

Sort paths and variables deterministically. If their number exceeds a small
display cap, report how many were omitted.

## What the evidence means

Let `H` be the vector of source-visible loop-carried values. A candidate
invariant `P(H)` must satisfy the familiar obligations:

```text
entry(H)                 ==> P(H)       establishment
P(H) && guard(H) && body ==> P(H')      preservation
P(H) && !guard(H)        ==> Q(H)       sufficiency for a desired continuation Q
```

Before a candidate `P` exists, there is no establishment or preservation
failure to classify. During specification inference there may also be no fixed
postcondition `Q`: inference is constructing a function summary rather than
proving a user-supplied one. The diagnostic must therefore report evidence for
the first two obligations, not claim that one of three obligations failed.

After the caller writes an invariant, ordinary verification already reports
the base and induction failures separately. Sufficiency is then expressed by
whatever assertion or postcondition remains unproved after the loop.

### Boundedness

Every `head[k]` fact is about paths which reach that head after exactly `k`
back-edges. It says nothing about `head[k+1]` or arbitrary states satisfying a
candidate formula. The report always includes the bound and the final warning
shown above.

A loop-local one-step relation is more generally useful than a list of bounded
heads because it shows what preservation must account for. That relation is a
planned extension. The implemented heads are still valuable because they expose
accumulated patterns and branch-dependent state without claiming induction.

### Exact, partial, and unavailable evidence

Each report has one of three statuses:

- **exact within the bound**: every reaching path in the bounded DAG was
  analyzed and every displayed carried value has a symbolic expression;
- **partial**: facts shown are valid, but a path, value, or memory effect was
  omitted because it could not be represented compactly; or
- **unavailable**: no useful source-level relation survived.

Partial output may omit facts; it must not weaken a path condition and present
the result as unconditional. An unknown value renders as unknown or is omitted,
never as an unconstrained equality.

## Soundness boundaries

### Do not read an invariant from an uninterpreted-predicate model

Introducing an uninterpreted predicate `I` is useful mathematical notation for
the missing invariant, but a Z3 model for `I` is not reachability evidence. In a
first-order validity query, Z3 is free to assign any interpretation compatible
with the asserted formula. It is not solving the second-order problem of
finding an inductive invariant, and a displayed function table can be arbitrary
or degenerate.

Consequently, the implementation does not assert `!I`, parse a model table, or
describe model entries as loop states. CHC solving or Spacer-based invariant
synthesis is a separate project.

### Do not emit bounded observations as loop invariants

A fact can hold on every path observed up to depth `N` and still fail induction
from an arbitrary state satisfying it. Even guarded rows such as
`i == 2 ==> r == old(r) + 2` are not generally preserved: a transition may
enter the guarded value from an unobserved state.

The first version emits diagnostics only. It does not add `Condition` values,
does not write a loop `spec` block, and does not use an
`[inferred = unrolled]` marker. A future source-edit feature may emit comments
or candidate clauses only if each clause is independently checked and clearly
distinguished from accepted inference output.

### Do not reinterpret ordinary WP annotations as forward states

`WPAnnotation` currently maps a code offset to conditions sufficient for the
suffix from that offset to reach a function exit. It is a backward obligation,
not the symbolic state reachable at that offset. Reading the annotation at a
copied loop header would reverse its meaning.

Loop-head evidence needs an explicit cut-point analysis described below.

## Analysis design

The evidence pass reuses the inference engine's expression and bytecode
semantics, but runs on isolated `FunctionData`. It never updates the function's
`Spec`.

### 1. Select eligible loops

Run normal inference first. A function is eligible when:

- at least one emitted condition is `[inferred = sathard]`;
- `LoopsWithoutInvariants` identifies at least one source loop; and
- the function is inside the requested inference scope.

This is correlation, not causal proof. With multiple missing loops, report each
as a possible source of precision loss. A `sathard` condition caused only by an
untrusted `result_of` carrier produces no loop evidence.

Extend `LoopWithoutInvariant`, which currently retains only `Loc` and
`is_inlined`, with a stable loop identifier, its original header, and the
source-visible subset of `FatLoopSpecInfo::{val_targets, mut_targets,
mem_targets}`. Preserve the pre-transformation data needed by the diagnostic
pass before `LoopAnalysisProcessor::transform` replaces the loop with havoc.

### 2. Build an isolated bounded DAG

For one eligible loop at a time:

1. clone the normalized pre-loop `FunctionData`;
2. force-unroll loops without authored invariants to depth `N` (the current
   implementation only proceeds when exactly one such loop exists);
3. abstract other missing loops in the ordinary way;
4. return a deterministic `k -> copied_header` map from unrolling; and
5. run diagnostic WP queries on cloned shadow data.

Refactor `LoopAnalysisProcessor::unroll` to return its head mapping instead of
discarding the local map. The ordinary pipeline ignores the mapping. The
evidence pipeline consumes it.

No unrolling mark, copied instruction, inferred condition, or annotation from
the shadow data may reach the primary target holder or generated source.

### 3. Compute cut-point relations

#### Implemented entry-to-head query

For each copied head, insert a synthetic `Stop` immediately after its label and
seed that exact instruction with:

```text
head[k].v == current(v)
```

In this diagnostic analyzer mode, ordinary returns, aborts, and other stops are
neutral exits. The existing branch-aware backward join therefore retains the
guards on paths which reach the selected stop; paths which leave the loop or
terminate elsewhere contribute no fabricated equality. The report explicitly
scopes every row to paths reaching that head.

The ordinary normalization and simplification path is reused, but model
mutation is disabled: the query does not call `update_spec`, common-subexpression
extraction, frame emission, or bad-temporary diagnostics. Expressions are
rendered through the Move sourcifier, and any condition which still exposes a
`$` temporary is omitted.

This produces establishment (`head[0]`) and accumulated bounded
entry-to-`head[k]` relations. It does not yet produce a transition universally
quantified over an arbitrary loop-head state.

#### Planned loop-local query

Generalize the internal WP analyzer with a diagnostic entry point which accepts:

- a start cut point;
- one or more end cut points; and
- a seed relation over named snapshot values.

For each carried value `v`, seed a fresh logical snapshot equality at copied
head `k`:

```text
snapshot[k].v == current(v)
```

Also compute `reaches[k]`: seed `true` at copied head `k` and `false` at every
other cut exit, then propagate that predicate to copied head 0. This distinction
is essential. Under ordinary partial-correctness WP, a path which exits before
the cut could otherwise satisfy the snapshot equality vacuously.

Run the existing backward transfer functions from copied head `k` to copied
head 0, stopping there instead of continuing to function entry. Rename the
state at head 0 to `head.v`, and guard every snapshot relation by `reaches[k]`.
The result is a path-conditioned relation between `head` and `snapshot[k]` for
executions which actually reach the cut. At `k == 1` it is the one-back-edge
transition; at larger `k` it is bounded accumulated evidence.

Run a second cut from function entry to head 0 to obtain establishment facts in
terms of parameters and function-entry memory. Keep these facts separate from
the loop-local transition relation.

The cut-point run must preserve the ordinary analyzer's branch-sensitive join,
abort semantics, mutation handling, and state labels. It must not call
`update_spec`, `cse_inferred_conditions`, or `emit_modifies`. A small result
type is sufficient:

```rust,ignore
struct LoopInvariantEvidence {
    function: QualifiedId<FunId>,
    loop_id: usize,
    loc: Loc,
    depth: usize,
    carried: Vec<LoopValue>,
    entry: Vec<PathRelation>,
    step: Vec<PathRelation>,
    heads: Vec<HeadEvidence>,
    completeness: EvidenceCompleteness,
    omissions: Vec<String>,
}
```

The current records live in a `LoopInvariantEvidence` annotation on the primary
function data. Only strings and loop metadata from the isolated analysis are
copied into that annotation; shadow bytecode is not. `SpecInferenceProcessor`
renders the annotation only when ordinary inference actually emitted a
`sathard` condition for the function.

### 4. Simplify for display

Use the inference simplifier on each relation, but only transformations which
preserve implication direction. In particular:

- retain path predicates;
- remove tautologies and duplicate equalities;
- do not solve a recurrence or guess a closed form;
- do not use solver-derived concrete values;
- prefer source names and source-level operations; and
- cap expression size, path count, and line count with explicit omission notes.

If the relation still contains an internal temporary, try the existing
substitution and source-name machinery. If it remains internal, omit that value
and mark the report partial rather than printing `$t17`.

## Placement in the current code

### Options and driver

- `src/inference.rs`: define the inference-only CLI field and copy its depth to
  internal prover options. Diagnostics continue through the existing model
  diagnostic/error-writer path; stdout/file/unified source remains unchanged.
- `bytecode-pipeline/src/options.rs`: carry an internal
  `Option<usize>` depth so processors can cheaply test whether evidence was
  requested. The public option remains grouped under `InferenceOptions`.

### Loop discovery and unrolling

- `bytecode-pipeline/src/loop_analysis.rs`: enrich
  `LoopsWithoutInvariants`, preserve the selected loop's pre-transform input,
  and expose the copied-head map from forced unrolling.
- `move-model/bytecode/src/fat_loop.rs`: factor construction of a
  `LoopUnrollingMark` so a diagnostic request can force a missing loop to
  unroll without adding a source pragma.

### Inference analysis

- `bytecode-pipeline/src/spec_inference.rs`: split analysis from model mutation.
  The ordinary path continues to apply the entry summary with `update_spec`;
  the diagnostic path calls the cut-point analyzer and returns evidence
  records only.
- No extra pipeline processor is required in the current slice. Loop analysis
  records the isolated result, and the existing inference processor renders it.

Avoid a second `GlobalEnv`: locations, symbols, types, and source maps already
belong to the primary environment. Isolation is at the owned `FunctionData`
and result-record level.

## Scope and degradation

The first implementation supports reducible, non-nested source loops
whose carried state can be named as locals, parameters, or mutable-reference
values. It should degrade deliberately:

- **Multiple loops:** currently report evidence unavailable; a later loop-local
  query can analyze them separately without false attribution.
- **Nested loops:** analyze the innermost loop first. Report the outer loop as
  unavailable in v1 if bounding the nested loop would multiply paths.
- **Inline-expanded loops:** retain the inline source chain. Prefer the existing
  `folds_of` repair guidance; show evidence only when carried values have useful
  source names.
- **Opaque calls or native effects:** preserve their existing symbolic summary
  when available. Otherwise mark affected values unknown and the report partial.
- **Global memory:** the current slice omits it and marks the report partial.
  A later version may report `global<T>(address)` only when the address and type
  are representable.
- **Nondeterminism or havoc inside the selected body:** quantify or mark unknown
  according to existing WP semantics. Do not turn a universal uncertainty into
  a concrete sample.
- **Early return or abort:** keep only paths which reach the requested copied
  head; state how many terminating paths were excluded.

## Cost and determinism

The ordinary inference run is unchanged. For an eligible target function,
requested evidence builds one bounded shadow DAG and runs `N + 1` cut-point WP
queries. Work grows with the unrolled DAG and branch count, so depth is capped
at 8 and displayed facts are capped at 8 per head. Dependencies are not
analyzed for evidence.

This design invokes neither Boogie nor Z3. Recorded output is deterministic for
a fixed source and depth. `stable_test_output` may normalize paths and names but
must not replace the useful relations with a redacted placeholder.

If a package-level inference deadline is introduced, evidence is best effort:
finish the ordinary inferred source first, then stop evidence collection with
an explicit incomplete note.

## Testing

### Soundness and isolation

- Enabling evidence leaves inferred conditions, frames, pragmas, and generated
  source byte-for-byte unchanged.
- Shadow unrolling and cut-point seeds never appear in the primary target data
  or model `Spec`.
- Every unconditional displayed equality is implied on all bounded paths which
  reach that head; branch-specific equalities retain their path predicates.
- Paths which terminate before a copied head are neutral exits in diagnostic
  mode; branch-aware joins retain the guards on paths reaching the synthetic
  cut. A future loop-local query should make `reaches[k]` explicit.
- Unsupported state becomes an omission or unknown, never a concrete value.
- No diagnostic contains raw temporary, label, or Boogie names.

### Baselines

The initial deterministic baseline covers two carried parameters, guarded
bounded heads, source-oriented rendering, omission notes, and the depth. Add
further inference-diagnostic baselines for:

- an additive accumulator (`sum_to_n`);
- geometric growth (`double_n_times`);
- a conditional body with two retained path predicates;
- a loop which exits before the requested depth;
- mutable-reference state;
- multiple missing loops;
- an inline-expanded fold;
- nested-loop degradation; and
- a non-loop `sathard` result which emits no loop evidence.

The baseline harness must not run a solver. Each focused test should stay below
20 seconds, including compilation in the normal warmed test environment.

### Analyzer checks

- Head 0 is the actual loop entry, not the function entry or loop exit.
- Head `k` corresponds to exactly `k` completed back-edges.
- One-step evidence is identical whether requested directly at depth 1 or as
  the first step of a deeper request.
- Ordering is stable across map/set iteration order.
- Expression and path caps produce explicit omission counts.

## Delivery sequence

1. **Done:** add the option, result types, and deterministic diagnostic renderer.
2. **Done:** preserve loop identity/carried targets and return copied-head maps
   without changing ordinary inference when the option is absent.
3. **Done:** factor model mutation from a non-mutating cut-point path and emit
   entry-to-head relations for `head[0..=N]`.
4. **Next:** add the generic loop-local one-step relation, explicit reach
   predicates, and separately attributable multi-loop analysis.
5. Evaluate whether accumulated heads materially improve invariant proposals.
   Do not add source edits unless that evidence supports a separately reviewed
   design.

## Evaluation

Measure the feature on novel loops, not the development-round tasks copied from
public inference fixtures. Evaluate at least:

- whether a caller proposes an invariant which passes both base and induction;
- turns and wall time from the first missing-loop diagnostic to a verified
  invariant;
- the fraction of requested reports which are exact, partial, or unavailable;
- evidence size and analysis time by depth; and
- false affordances: cases where a caller mistakes a bounded pattern for a
  proved invariant.

A useful result is not merely a plausible formula. It is a reduction in effort
to reach an invariant that the ordinary prover independently verifies.

## Non-goals

- invariant synthesis or CHC solving;
- automatic insertion of loop `spec` blocks;
- changing the meaning of `[inferred = sathard]`;
- enabling the feature by default in verification or inference;
- changing MoveFlow or its evaluation arms as part of the core implementation;
- using bounded evidence as proof beyond the requested depth.
