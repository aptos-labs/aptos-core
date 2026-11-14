# Nominal closed and open sorts for data invariants

> **Closed on 2026-11-13, not applicable.** The migration this document plans
> was stopped, and the carrier model it preserves is being withdrawn. The
> forward plan is [`plain-values.md`](plain-values.md); the measurements behind
> the decision are in [`transitions.md`](transitions.md), "What it cost, and
> the decision to withdraw it". The text below is kept for its rationale and is
> not updated. What carries over from the implementation is the nominal type
> identity, listed in the forward plan.

## Objective and scope

Represent a declaration's valid values by a nominal closed sort (`Percent`),
and its temporarily mutable values by a distinct nominal open sort
(`Percent$Mutated`). Validity is a theorem about every inhabitant of the closed
sort. Verification terms must not expose a subtype predicate or carry a
certificate inside each value expression.

This is a representation and proof-interface migration, preserving the carrier
model's semantics: typed storage, invariant obligations at construction and
mutation boundaries, prophetic references, and the agreement with source
execution. It does not revert to a whole-memory invariant at every write or
join. It does not remove hand-over, copy-out, or exit obligations merely to
recover performance.

This proposal replaces the exposed-subtype certification section of
[`native-nominal-types.md`](native-nominal-types.md). That plan's atomic type
identity, native payloads, codecs, models and generation remain useful. Giving
only the payload a nominal name while retaining a visible subtype around it
does not meet this plan's objective. Current behavior remains documented in
[`data-invariants.md`](../designs/data-invariants.md) until implementation.

Preserving native spelling is a requirement of this migration. The closed
representation changes; the ADT operation vocabulary and small named type
applications remain. Direct reduction through a closed value's private
representation may be replaced by proved operation laws, whose cost must be
measured rather than assumed negligible.

The motivating evidence is Fable's
`/home/wrw/shared/handoff/verex-orderedmap-cost-analysis-2026-11-10.md`, especially
sections 1d, 4 and 5. It reports larger implicit arguments, proof-dependent
rewriting, repeated certificate extraction, and multiple spellings of values.
Its figures concern recorded revisions, not the current worktree; they are
reference points, not a measured speedup for this proposal. The source file is
an external handoff, not a build dependency.

## Required interface

For each supported declaration and argument interpretation, provide named
closed and open sorts and named operations. The following is schematic;
`openValue` avoids using a Lean keyword:

```lean
Closed : Type
Open : Type
valid : Open → Prop
openValue : Closed → Open
closeValue : (x : Open) → valid x → Closed

valid_open : ∀ p, valid (openValue p)
open_close : ∀ x h, openValue (closeValue x h) = x
close_open : ∀ p, closeValue (openValue p) (valid_open p) = p
```

The laws imply injectivity of `openValue` and independence of closing from the
chosen proof. They express precisely the correspondence between closed values
and valid open values. Construction cannot introduce arbitrary inhabitants of
`Closed`; its public operations must preserve these laws.

The implementation must construct and prove this interface in Lean, without
new axioms or admissions. A private subtype implementation is permissible as
a proof construction only if the measured execution and solver interfaces keep
it hidden. A reducible alias of a subtype is not a nominal boundary. Stage 1
must settle how actual distinct sorts, their operations and their transparency
are generated; an opacity annotation alone is not evidence of low cost.

No unconditional `Inhabited Closed` is introduced: an invariant can be false.
Preserve the existing satisfiability hypothesis and its public assumption
ledger wherever default inhabitants are required.

### Continuations contain values, not certificate applications

Do not implement the migration by replacing `ofDeepHolds x h` with an opaque
`closeValue x h` everywhere. The latter still contains `h` as a term argument.
Prove a weakest-precondition rule whose obligations have this shape:

```text
construction or closing of x:
  prove valid x
  continue with p : Closed and e : openValue p = x
```

More precisely, a continuation premise can be
`∀ p : Closed, openValue p = x → Q p`; together with `valid x`, the interface
laws justify applying it to the concrete closing operation. Prove the rule
against the existing certifying semantics, including its undefined behavior
when validity fails. Its generated continuation binds `p` and `e`; the closing
proof occurs in the enclosing proof of the transition, not in every use of `p`.
Keep this equation oriented as an observation of `p`; never rewrite `p` back to
`closeValue x h` in normal verification terms.

This is a solver-facing rule over the existing semantics, not an unexplained
fresh-value operation or a new assumption. Measure its own cost: naming a value
and adding an equation may still be expensive.

### Canonical ADT operations

Declarations with their own data invariants are ADTs: structs and enums. Generate
one canonical vocabulary of constructor operations, projectors and
discriminators for each declaration instance, shared by executable terms,
contracts, invariant clauses and solver rules. For example, closed reads spell
`Percent.value p` and `Payment.isDirect p`; open updates act on the native
`Percent$Mutated` payload. Operations must not expose structural rows or
alternate between native and descriptor-based spellings.

Generate and prove the following laws from the declaration's native model:

- Projection after construction and the corresponding open-update equations.
- Discriminators for each constructor, constructor disjointness and injectivity
  on valid inputs, and equality/extensionality for closed values.
- Exhaustive case elimination for closed enums, with payload observations and
  validity facts available in the selected branch. Variant-specific projectors
  are used under the matching discriminator or branch evidence; no validity
  fact is invented for an unselected payload.
- Agreement between closed observations and observations of the opened value,
  including nested ADT fields and their validity.

An invariant-bearing closed constructor is not an unrestricted total function
from arbitrary fields. Its canonical construction operation incurs the validity
obligation and uses the fresh-closed-binder rule above. The resulting binder's
constructor/observation equations are proved consequences of that transition.
Do not expose a constructor proof argument in subsequent value expressions, or
introduce an unconditional constructor that silently changes invalid inputs.

Choose one orientation for each observation law so that ordinary closed reads
stay at their canonical projectors and boundary equations reduce observations
of a newly constructed value to its fields. Opening is needed at mutation
boundaries; ordinary reads must not accumulate `view (open (close ...))`
conversions. Proved elimination rules supply native case reasoning without
requiring the solver to unfold the closed representation.

Containers can inherit validity from ADT elements even though they declare no
own invariant. Their native selection/update laws must compose with the ADT
interface. Scalars and abstract generic parameters retain their existing
representations; the ADT observation interface does not replace those models.

### Observations, fields and nesting

- Define one canonical observation spelling per operation. Closed field reads
  use named observers; open updates use named open operations. Do not alternate
  between observers, subtype projections and `forget` chains.
- Prove observer equations, equality/extensionality, constructor and match
  laws, and the validity facts of closed nested fields. Consumers use these
  laws rather than destructuring an implementation wrapper.
- Open payloads allow nested declarations to be open as well. Closing restores
  both the owner's own invariant and required descendant invariants, including
  elements of vectors, tables and enum payloads. An owner with no own clause
  may still have nontrivial validity because of its fields.
- Specification observations use the same named native observations and
  predicates. Reading or opening an invariant must not encode a value, decode
  it, or instantiate a new carrier family.
- Expose invariant clauses when needed, once per canonical value and relevant
  path context. Do not scan and re-open every local at every normalization.
  Facts must not survive outside the branch in which they were established.

### References, generic parameters and storage

Keep mutable references as an open current and a closed prophecy. Preserve the
existing loan-death ordering: prove validity before equating the current with
the prophecy's observation. Open sorts do not replace ownership or liveness
analysis. Lean's type checker prevents passing an open value where a closed one
is required; it does not enforce linear use of an opening operation.

Do not add two abstract sorts to every Given parameter. The existing generic
body cannot open an uninterpreted `T`; its canonical single-sort treatment
remains as described in [`given-parameters.md`](../designs/given-parameters.md).
Declared generic ADTs still have distinct open and closed sorts. Concrete
instantiation and relational transfer must preserve the new interface, using
named instantiated operations rather than repeating family substitutions.

Typed memory continues to hold closed values. Storage decoding must produce a
closed value only when validity holds. Prove encoding preservation under
opening and closing, codec round trips and the laws used by `StorageEncodes`.
Runtime bytes do not acquire proof fields. Preserve `GlobalsPreserved`,
`FailuresIntegral`, satisfiability, and existing transfer assumptions without
silently strengthening or dropping them.

## Implementation stages

Each stage is a coherent, buildable landing. Existing unrelated work must remain
intact. Temporary adapters are migration mechanisms, with an explicit removal
stage; do not establish a second permanent semantic pipeline.

### 0. Establish a reproducible baseline and dependency inventory

Record the exact source revision and any local diff in benchmark artifacts.
Use one native execution mode and comparable complete theorem totals, including
setup and authored proofs. Record stage profiles separately; solver walk totals
are not complete proof totals. Distinguish timeouts and undecided goals from
completed measurements.

Inventory representation dependencies by role, not a blanket replacement of
`Subtype`: carrier definitions; `.val`/`.property` field operations;
`ofDeepHolds`/`forget`; subtype equality and destructuring; contract emission;
codec and reframe proofs. Other uses of subtypes, such as index bounds, are
outside scope.

Primary source areas:

- `VerexIR/Proofs/Denote/Types.lean`, `Term.lean`, `Resources.lean`,
  `FunctionValues.lean`, `Interpretation.lean`, `GivenOwn.lean` and
  `Agreement.lean`: representations, operations and semantic laws.
- `VerexSL/Verify/FunctionValues.lean`, `Contracts.lean`, `Members.lean`,
  `VerexSL/Contract.lean` and its submodules: generated declarations, contracts
  and theorem binders.
- `VerexIR/Proofs/Denote/Compile.lean`, `Solve.lean`, and
  `Solve/{Normal,Base,Decide,Invoke}.lean`: producer operations, cuts,
  observations, invariant exposure and equality.

Deliverable: baseline artifacts and a bounded inventory of dependencies to
migrate, including the current status of native nominal emission.

### 1. Prove and measure a complete small interface

Implement a hand-written nominal interface for a scalar struct with an
invariant, a nested generic struct, and an enum. Include a declaration without
an invariant. Prove the interface laws, closed field validity, native
observation laws and the fresh-closed-binder WP rule.

Exercise construction, matching, a temporarily invalid mutation repaired
before loan death, and a nested field read. Inspect generated expressions under
the actual normalization and decider pipeline. Require that continuation value
types remain named, and value expressions contain neither subtype projections
nor closing proof applications for these declarations.

Compare with the current implementation on the same examples. Record term
sizes, normalization cost, complete proof cost and final proof-checking cost
where available. Private proof representations may still affect elaboration
and kernel checking even when leaves are small.

Exit gate: actual nominal separation, proved semantics, and evidence that the
interface survives the production solver without exposing its representation.
Specifically compare constructor/projection reduction, nested field reads,
discriminators and exhaustive enum matching with the existing native model.
Require one canonical operation spelling from every producer, and account for
the cost of theorem-based reduction at the closed boundary. Smaller type terms
alone do not pass the gate if observation or case reasoning becomes expensive.
If the prototype merely relocates the same costs, revise it here before a
tree-wide migration.

### 2. Generate nominal interfaces and integrate the carrier family

Extend the native nominal generator and models to produce both sorts, native
validity predicates, the canonical ADT operations and their named laws. Retain the existing plain
payload work where compatible. Cover currently supported finite declarations,
type arguments, nested ADTs, enum payloads and ghost fields. Preserve explicit
diagnostics for unsupported shapes; recursive-type support is not a prerequisite
or an excuse to broaden this migration.

Add the closed/open interface to the carrier interpretation. Initially provide
proved bridges to the current representation so unaffected operations continue
to compile. Establish codecs, equality, order, ghost erasure, parameter
instantiation and Given-to-concrete transfer laws. Native layout metadata must
not be copied into every type mention.

Exit gate: generated fixtures have real distinct sorts and all bridge laws are
proved. No new public theorem assumptions; bridges are absent from the small
prototype's generated verification conditions.

### 3. Move operations and boundaries onto the interface

Migrate construction, variant matching, projections, open updates, local and
global borrowing, loan completion, copy-out, hand-over and all normal exits.
Use the fresh-closed-binder rule at closing cuts. Keep obligation provenance
and source diagnostics, including explicit early returns.

Keep the current obligation locations first. For declaration instances whose
whole validity predicate is proved trivial, generate a direct construction or
copy rule without a validity cut. Test triviality at the full instance, not
just whether the declaration has an own clause. Avoid expensive rediscovery of
triviality at each use. Further elimination of duplicate cuts requires a
separate proved rule and measurement.

Exit gate: positive mutation examples pass; invalid packing, loan death,
hand-over, copy-out and returned references fail at the expected source site.
Storage remains closed throughout observable execution.

### 4. Migrate contracts, solver observations and generic calls

Emit nominal closed binders for arguments, call results, loop-carried values
and stored reads, with open binders only where semantics permit mutation.
Specifications observe these binders through canonical native operations.
Apply native validity laws to closed values and open selected clauses without
the encoding/decoding route.

Replace carrier-specific subtype recognizers, certificate splitting,
projection reconstruction and equality repairs with interface recognizers and
named laws. Register only deliberately oriented observation equations; global
unfolding of sort implementations or all invariants is forbidden.

Check concrete generic instantiation, Given bodies, contract-routed and inlined
calls, and loop frames. Preserve the canonical parameter representation and
existing row-route gains. A distinct sort must not force repeated reframe
casts into ordinary field observations.

Exit gate: expression audits cover parameters, constructions, call results,
loop binders and memory reads, not only isolated leaf goals. Performance gates
below decide whether the symbolic walk as well as its leaves improved.

### 5. Remove adapters and validate the complete path

Remove the old exposed-subtype ADT representation, migrated representation
repair rules, and compatibility adapters. Keep unrelated bounded types intact.
Audit remaining `Subtype` and certificate references by semantic role.

Run the full verification matrix, compare complete benchmark totals and
assumption ledgers, and update the current design documents to describe the
implemented model. Resolve the superseded certification section of the native
nominal plan. Preserve unsupported-feature diagnostics and authored proof
behavior; record necessary authored-proof migrations explicitly.

## Validation and performance gates

Correctness coverage must include:

- Lean rejection of an open value at a closed argument or storage position,
  with no automatic open-to-closed coercion.
- Valid construction and rejection of invalid construction; false invariants
  must not manufacture inhabitants or make verification silently vacuous.
- Temporary violations repaired within a loan, nested loans and reborrows,
  invalid copy-out and hand-over, explicit return, and abort paths.
- Generic/nested ADTs, enum branches, vectors and tables, field replacement,
  loop joins, closed equality and specification reads.
- Canonical constructor, projector and discriminator spellings across code,
  contracts and invariants; constructor injectivity/disjointness, exhaustive
  enum elimination and branch-scoped payload facts. Audit nested reads for
  conversion chains and implicit expansion of layouts or certificates.
- Typed storage reads/writes and same-key read-after-write behavior; existing
  source agreement and interpretation transfer theorems still compile.
- Generated term audits distinguishing the continuation and its value terms
  from the enclosing proof of the transition. No assumption is accepted merely
  because a metaprogram generated a well-typed declaration name.

Use these benchmark groups at baseline, after the prototype, after solver
integration and at completion:

| Group | What it decides |
|---|---|
| `make_pair`, `make_transfer`, `total`, `read_pair` | Fixed overhead where no invariant exists; construction, matching and observation |
| OrderedMap `empty` | Native predicate and closing-cut cost without a call |
| OrderedMap `borrow_key_at`, `lower_bound` | Diffuse representation cost in setup, leaves, exits and generic observations |
| OrderedMap `missing_scenario`, `lookup_scenario` | Composition, repeated boundaries and retained row-route gains |
| OrderedMap authored `add`, `remove`, `lower_bound_loop` and removal scenarios | Complete authored-proof cost and unresolved invariant obligations |
| `bitwise_table::add_fail_exists`, `mut_ref` | Regressions previously caused by splitting certified binders |
| `primary_fungible_store::deposit` and framework sample | Typed-storage benefit and whole-pipeline behavior |
| `DenotePerformance`, `CompositionPerformance`, standard benchmark | Broader regressions and build/proof-size costs |

Charge invariant work by `CertifyKind` and separate the validity obligation
from continuation preparation. Report wall time, heartbeats, completion status
and peak memory where available. Compare native runs with native runs and full
proofs with full proofs; keep stage profiles as explanatory measurements.

Acceptance targets, to be evaluated against newly reproduced comparable runs:

- Invariant-free targets should be within roughly 15% of the pre-carrier cost,
  following Fable's proposed diagnostic threshold. If the historical revision
  cannot be reproduced, report that limitation rather than claiming this gate
  passed. Also compare against the current baseline.
- `empty` should use one semantic closing cut with a native predicate, without
  repeated encoding/frame-opening visits. No unconditional numerical speedup
  is assumed from changing the type name.
- `borrow_key_at` must show improvement in full cost, including setup and exit,
  not just reduced certificate-extraction time. Its solver terms must satisfy
  the representation audit even if another stage becomes dominant.
- Existing passing targets must remain passing at their configured budgets.
  Any regression needs a stage-level explanation before rollout. Existing
  undecided goals remain separately tracked; a timeout is not a successful
  performance result.
- Typed-storage examples must not regain memory-invariant preservation proofs
  at every write and join, and no existing soundness boundary may be weakened.

Run the four package builds and tests for shared semantic changes, following
`CLAUDE.md`; use the owning suites during isolated prototype work. At completion
run the Move Prover Lean gate (`MVP_TEST_FEATURE=lean cargo test -p move-prover
--test testsuite`) as well. Baselines are regenerated only through their owning
tools after inspecting expected diagnostic changes.

## Complexity and principal risks

The new work is nominal interface generation, proof-preserving transition
rules, and removal of representation dependence in producers and consumers.
The existing `forget`/`ofDeepHolds` correspondence, boundary analysis and typed
storage are reusable. This is smaller in semantic scope than introducing
carriers, but crosses the type interpreter, generator and solver.

The early prototype must resolve transparency and proof-term cost. Subsequent
risks are casts at generic boundaries, recursive opening of nested values,
abstract-layout map models, match/equality automation and hidden assumptions
about inhabitants. Compile-time sort errors help locate misuse in ordinary
Lean code; generated-expression errors surface only when their fixtures run.
Neither substitutes for the semantic and performance gates above.
