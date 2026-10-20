# Shared obligations and incremental preparation

Implementation plan for P3 and P4 of
[`proof-strategy.md`](proof-strategy.md). The implementation stages below are
planned; the existing authored `verex_obligations preparing` hook is the
starting point. Track progress in [`roadmap.md`](roadmap.md).

## Objective and limits

Keep a symbolic path's formulas, assumptions and derived facts shared while
deciding its obligations. Do cheap common work before splitting, and make
children process the facts their branch adds. Automation should finish easy
proofs cheaply and report hard obligations for authored proofs.

Preserve the existing 20M cap for a complete automatic leaf attempt and the
overall target budget. Authored proofs continue directly to their script,
without speculative automatic proving. Specifications, supported outcomes,
source clause names and the kernel-checked proof boundary remain intact.
This plan introduces no external solver or general SAT solver.

The original completion criterion remains **all 38 supported BigOrderedMap
targets verified below roughly 1.9G total native heartbeats**, including their
authored proofs. The reference total after shared authored preparation is
3.582G; the original reference was about 19–20G. DAG sharing is a proposed
means of reducing that cost, not evidence
that the target is already attainable through this change alone.

## Evidence and questions to resolve

The current walker in
[`Solve.lean`](../verex-ir/VerexIR/Proofs/Denote/Solve.lean) splits conjunctions,
introduces binders and splits variants/conditionals. `leafStep` then invokes
`solveLeaf` independently on each automatic leaf. `splitOnce` can invoke
variant splitting and context normalization before the leaf's cheap deciders.
There is no separate general CNF conversion followed by a resolution engine.

Lean expressions already support shared subterms. Verex caches some normalized
hypotheses and conditional rewrites, names continuations, and skips known
normal subterms. Lean's `grind` also hash-conses expressions and maintains an
e-graph within an invocation. These mechanisms do not retain the full prepared
or theory state between our sibling leaf attempts.

Lightweight automatic profiles motivating the plan; M means one million
raw heartbeats, and the totals here measure the solver only:

| OrderedMap target | Leaves | Split stage | Leaf solving | Solver total |
|---|---:|---:|---:|---:|
| `test_verify_iter_collect_symbolic` | 51 | 58.869M | 79.325M | 172.798M |
| `test_verify_iter_sum_symbolic` | 35 | 42.224M | 75.951M | 148.480M |

The split stage includes preparation, so it is not a measurement of Boolean
formula construction alone. Map-position preparation runs 42 and 27 times,
costing 19.717M and 17.647M respectively within the leaf work. Invocation counts
do not establish that all of those calls are redundant. Nested tactic costs
overlap and must not be summed.

Verbose-debug replays can change traversal counts and exhaust leaf budgets;
exclude them from comparisons unless their observer effect is understood.
Recreate probes from the rendering under test with the same callee theorems.

Prior experiments in [`perf-notes.md`](perf-notes.md) constrain the approach:

- Sharing authored preparation before clause splitting already reduces the
  OrderedMap mutable-walk replay by 28.8% and the BigOrderedMap predecessor
  replay by 17.7%.
- Traversing encoded-map terms as a DAG alone gave no saving. Sharing full
  simplifier state and cross-target normal forms also gave negligible gains.
- Removing only `forall_and` modestly reduced generation cost, retained many
  leaves and broke clause routing in leaf-sum. Do not repeat it as a blanket
  normalization change.

## Intended representation

Use a shared formula graph together with a tree of assumption scopes. Formula
identity and validity of a fact in a context are separate concerns.

| Record | Contents and lifetime |
|---|---|
| Path context | Parent context, local declarations/instances, additional assumptions, context revision, prepared facts and consumed preparation budget; owned by one verification attempt. |
| Formula node | Shared Lean expression with its binder scope; retain existing local definitions for repeated computations. Introduce a separate intern table only where measurements justify it. |
| Obligation | Formula reference, context reference, provenance, all source clause occurrences and proof reconstruction destination. |
| Preparation result | Derived fact and proof, dependencies, context revision and stage already completed; reusable only where those dependencies remain valid. |

Prepare common hypotheses once. Store target-specific facts separately, and
derive them only for terms relevant to a pending obligation. After introducing
a clause's quantifier, prepare the new local facts without redoing the parent.
After a substitution, invalidate affected entries; a single inherited
`prepared : Bool` is insufficient for incremental preparation.

Initially reuse facts only within the same local context and its descendants.
Reuse across renamed contexts requires explicit proof transport, including
local definitions, instances and auxiliary declaration names, as the existing
context-grouping implementation requires. A formula hash never establishes
that a proof from another context is applicable.

Reconstruct ordinary Lean proofs using conjunction introduction, binder
abstraction, case analysis and proved rewrites. Shared subproofs can be named
and referenced by multiple obligations. An obligation still owed must never
become an assumption for its siblings. Sharing a formula must retain every
source occurrence for diagnostics.

## Implementation stages

### 0. Establish a reliable cost breakdown

Add optional lightweight counters around `splitStep`, `splitFirst`,
`solveLeaf` and the context-preparation stages. Separate structural goal
creation, variant/conditional splitting, normalization, fact derivation and
leaf decision cost. Count distinct contexts, added facts, repeated preparation
requests, unique formula nodes and visits to those nodes. Keep pretty-printing
out of this instrumentation.

Record whole-theorem heartbeats and proof-term/kernel cost alongside solver
cost. Check that instrumentation preserves verdicts and traversal counts
against an uninstrumented replay. Investigate the verbose-debug discrepancy
only as needed to make these measurements trustworthy.

Use the two automatic targets above, OrderedMap's mutable walk, and
BigOrderedMap's predecessor, leaf-sum, upsert and collection. Include cheap
arithmetic, table-read and vector controls, plus false clauses. Retain complete
callee context when isolating targets.

**Exit:** a reproducible baseline identifies which preparation and branch work
is repeated across siblings. Pure expression traversal, normalization and
decision-procedure cost are reported separately enough to choose the next
change. No solver refactor is required to complete this stage.

### 1. Share bounded preparation in automatic mode

Reuse the completed-path boundary of `prepareObligations`, after path binders
and program continuations are processed and before proof clauses split.
Extract a small common preparation operation callable by the automatic walker
and authored preparation; do not invoke the complete `verex_prepare` or
`verex_leaf` cascade on every path.

Start with assumption/reflexivity checks where applicable, hypothesis
destructuring, safe variable substitution, projection reduction, type bounds
and normalization of changed hypotheses. Derive map facts only for relevant
terms, with the same typed-map interface used by authored proofs. Leave
speculative quantifier instantiation, variant enumeration and general search
to the later decision step.

Children inherit proved facts and stage/dependency information. Revisit only
new or changed facts. A preparation attempt that does not help restores its
tactic state and dependent caches; its spent heartbeats remain charged and it
is not retried unchanged in every child. Keep an internal comparison switch
during development, then retain the cheaper path selected by the results.

**Exit:** both automatic probes still verify, expensive common stages execute
once per applicable context revision, whole-theorem costs fall, and cheap
controls stay within their existing gates. The first two implementation
stages aim for at least a 20% reduction in the probes' combined theorem cost;
that is an experiment target, not a forecast. Record smaller gains accurately.

### 2. Delay logical and variant splitting

Separate traversal of program control flow from decomposition of completed
proof obligations. Keep a path's pending clauses together through common
preparation and cheap decisions. Preserve quantified conjunctions where doing
so shares assumptions, introducing a clause-specific binder when its proof
needs it.

When a remaining group needs a case split, choose one relevant discriminator,
branch once for that group, and propagate its assumption to the affected
clauses. Clauses independent of that discriminator stay outside the branch.
Process children lazily instead of materializing the product of every tested
local's variants. Do not reorder dependent program steps or their obligations.

Keep source markers separate from simplifier shape. Existing `verex_on`,
`verex_paths`, plain obligation generation and authored `preparing` blocks
must still receive the obligations they address. Avoid generating, combining
and reopening identical goals just to recover shared preparation.

**Exit:** repeated iterator case analysis falls in the map probes, cases can
close before producing all leaves, and quantified/source-named clauses retain
their meaning and routing. Any lost automatic positive gets an analysis of
whether the new traversal missed cheap reasoning; hard reasoning belongs in
an authored proof, not a larger search budget.

### 3. Make formula and proof sharing explicit where needed

Use stage 0's node/visit counts after stages 1–2. If repeated formula traversal
or copying remains material, introduce the context/formula/obligation records
above and migrate the pending queue incrementally. Reuse Lean expressions and
existing normalization caches before adding another term representation.

Preserve conjunctions and conditional nodes as shared structure. If a Boolean
consumer requires clauses, give compound subformulas names with checked
definitions or equivalences, in the style of Tseitin encoding, instead of
distributing disjunction over conjunction. Parameterize definitions by their
free variables and keep them in the correct binder scope. Do not introduce a
fresh unconstrained proposition as a substitute for the original formula.

Cache proofs as well as normalized forms where the dependencies permit it.
Check the exported theorem's size and kernel cost: a compact meta-level DAG
that expands into a large proof term is not sufficient.

**Exit:** a repeated-subformula fixture scales with distinct nodes for the
graph construction and traversal being optimized, existing formulas produce
kernel-checked proofs, and real targets improve in total heartbeats. This gate
does not claim that proof search becomes linear. Defer this stage if remaining
cost is in mathematical reasoning rather than formula processing.

### 4. Reuse theory state only if it remains a measured bottleneck

First retain proof-backed derived facts and term indexes between siblings.
Measure what arithmetic internalization, congruence reasoning or quantified
instantiation is still reconstructed by independent solver calls.

If that work dominates, prototype one prepared solver context with persistent
branch snapshots, adding only branch assumptions and the current obligation.
Evaluate Lean's existing `grind` interfaces before introducing new machinery.
Reuse a theory state only with compatible local variables, instances and
metavariable assignments. Branch-specific equalities, substitutions and
instantiations must not escape into siblings. Check that adding all clauses to
one invocation does not increase irrelevant search.

**Exit:** a matched experiment demonstrates lower total cost at the unchanged
budgets, including snapshot maintenance and proof reconstruction. Retain the
earlier stages independently if an incremental theory backend does not help.
This stage is conditional; no solver rewrite is a prerequisite for stages 1–2.

### 5. Simplify authored proofs and validate the corpus

Apply the shared path preparation to the remaining expensive BigOrderedMap
and OrderedMap proofs. Remove repeated cases, normalization, bounds and map
lifting only when the shared context now supplies them. Keep direct map laws
and arithmetic arguments readable. Then review the other companion files for
the same pattern, introducing direct proofs where bounded automation still
spends its allowance on hard goals.

Rank remaining costs by obligation generation, preparation, authored tactics
and kernel checking. A local optimization is complete when its own gates pass;
the original performance goal is complete only when the native BigOrderedMap
total meets the stated threshold. If it remains above that threshold, record
the remaining cost owners and continue targeted proof work rather than
claiming the graph architecture solved the original problem.

## Budget accounting

Shared automatic work must not create an unbounded prepass or grant a fresh
20M allowance whenever an attempt splits. Give common preparation a small
internal slice of the existing allowance, selected using stage 0's controls;
there is no new user-facing budget setting or increase in total allowance.

Track consumed preparation along context ancestry. Each independent leaf's
remaining allowance is its configured cap minus the common automatic work
charged to that leaf. Descendants created inside one leaf's solver attempt
share that attempt's remaining allowance. Failed speculative work stays
charged after restoring tactic state. Report physical heartbeats once in the
performance totals, even when a shared cost limits several leaves' allowances.
The overall target budget continues to bound the complete verification.

Authored obligation generation and its explicit preparation block remain under
the target budget. They do not acquire automatic leaf accounting unless the
author explicitly invokes the automatic solver. Test low configured caps and
the existing zero/unlimited convention as well as the default.

## Validation and rollout

Extend [`SharedPreparation.lean`](../verex-ir/VerexSL/Tests/SharedPreparation.lean)
and add focused automatic/graph fixtures as their stages land. Cover shared
cases, contradictory branches, a false residual conjunct, the same formula
under different assumptions, quantified and dependent binders, local
definitions/instances, substitutions, folded continuations and callee/loop
clause names. Include budget exhaustion, failure rollback and a proof whose
unproved sibling would make it spuriously succeed. Negative verification
fixtures must remain rejected, and unavailable proofs must never yield a
verified theorem.

During iteration, run affected targets and `scripts/smoke.sh`, with
`SharedPreparation`, `ArithmeticContext`, `TypedMaps` and the applicable
read-only performance gates. Do not run the full suites after every edit.
Never rebuild shared Lean libraries while proof replays or benchmarks use
them; use a scratch build for instrumentation experiments when necessary.

At a stable checkpoint, run the native complete map measurements and the
complete benchmark, then regenerate `local_benchmark.{json,html}` against
main and validate the report. The manifest excludes BigOrderedMap, so report
its total separately. Native reference measurements are OrderedMap 29/29 at
1,430,479,730 heartbeats, BigOrderedMap 38/38 supported at 3,582,087,580, and
the corpus benchmark at 9,787,642,404 with 30 verified and AMM's expected
rejection. Keep the five existing BigOrderedMap unsupported diagnostics
separate from proof failures.

Once the implementation stabilizes, run all four Lean suites and the full
Move Prover Lean gate once. Inspect every changed diagnostic and
update performance baselines only for measured, intended changes. Record the
final costs, proof coverage and HTML artifacts in the PR or benchmark
results, and deferred stages in the roadmap. Documentation-only planning
requires no proof test run.
