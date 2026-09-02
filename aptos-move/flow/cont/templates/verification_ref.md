{% if once(name="verification_ref") %}

{% include "templates/spec_editing_ref.md" %}
{% include "templates/spec_lang_proofs.md" %}
{% include "templates/toolchain_limits.md" %}

## Move Prover reference

Call `{{ tool(name="move_package_verify") }}` with `package_path` and an explicit
timeout. Its optional controls are:

- `filter: "module"` or `"module::function"` for a focused proof;
- `exclude: [...]` to omit known targets temporarily while diagnosing others;
- `split_vcs_by_assert: true` to identify which assertion in a function is hard
  or false;
- `error_limit` to bound counterexample output.

Filters and exclusions are diagnostic conveniences. A final proof must cover
the user's requested scope, and an unmatched or excluded target is not success.

### Reading a counterexample

A counterexample shows the frames of one failing execution with the values the
solver chose. Read the notation before drawing conclusions from it.

- **Named locals** appear under their source names, and `result` is the return
  value. Reason from these first.
- **`$t<N>`** is a compiler- or prover-introduced temporary with no source
  counterpart. Read it as an intermediate value at that step; do not look for
  it in the source or name it in a specification.
- **A frame marked `(spec)`** lies inside the function's spec block, so it
  evaluates a condition rather than executed code.
- **`<generic>`** is the value of a type parameter, withheld because it cannot
  affect the outcome.
- **A function value** prints as the source entity it came from. A closure
  shows the function it packs and its captured arguments by parameter name.
  `<value of function field ...>` and `<value of function parameter ...>` are
  values the solver chose for that field or parameter, and the trailing `#n`
  distinguishes distinct values of the same field, so a repeated `#n` is the
  same value. Their behavior is only what the specification states about the
  carrier, so give it exact `result_of` and `aborts_of` conditions.
  ``<some `T`>`` is a function of type `T` the solver picked with nothing
  tying it to a source entity.

### Classify before editing

- **Compilation/spec-language error:** fix syntax, name resolution, placement,
  or invalid `old()` use before reasoning about the proof.
- **Postcondition counterexample:** trace the normal path. Determine whether the
  implementation violates the intended contract, a callee contract is too weak,
  or a loop invariant loses the needed fact.
- **Abort counterexample:** enumerate direct and transitive aborts, including
  arithmetic, indexing, resources, and opaque callees. Complete the exact abort
  behavior; do not turn on partial abort checking.
- **Frame failure:** compare executable global writes with `modifies` clauses,
  especially across opaque callees.
- **Invariant failure:** separately check initialization, preservation, and the
  loop-exit implication. A stronger invariant is useful only if the body proves
  it.
- **Timeout/out of resources:** treat the contract as unresolved, not false and
  not verified.

Never make a desired property disappear to obtain a green prover result. A new
precondition is valid only if it reflects the intended API, not because it
excludes the counterexample.

### Reading timeout analysis

A timeout diagnostic carries evidence from a replay: the prover re-runs the
captured query under a profiling solver instead of reporting the original run.
The counts therefore describe the same obligation without being exact, and a
`+` marks a lower bound.

- **Quantifier activity** ranks what the solver instantiated, naming a source
  location for each entry. Reduce the instantiations that entry needs instead
  of raising the budget. A `definition of spec function` entry points at that
  helper: align its recursion with the loop so one obligation unfolds one
  step, and keep the recursion single. A `forall` entry points at a written
  quantifier: give it a valid trigger, or replace it with a frame or a
  bounded relation.
- **Nonlinear arithmetic activity**, reported through the `arith-nla-*`
  counters, means the search is in nonlinear arithmetic. Prefer additive
  recurrences to closed forms and keep symbolic products out of invariants.
- **Mixed activity** reports both. Address the top named quantifier first and
  then the arithmetic; the two compound.
- **Incomplete or partial evidence** still ranks the quantifiers, but the
  classification is unsettled. Do not read a missing counter as evidence that
  its cause is absent.
- **Unavailable evidence** means the replay could not run. Fall back to
  isolating the obligation with `split_vcs_by_assert` and a narrower filter.

A named source location is the place to change. A timeout leaves the contract
unresolved either way, so never answer one by weakening it.

### Timeout strategy

Where the analysis names a definition, aim the work there; otherwise work
from the smallest failing function. Preserve contract meaning:

1. Simplify WP-generated or hand-written expressions: remove proven redundancy,
   factor common terms, replace mechanical updates, and repair vacuous or
   `sathard` loop output.
2. Use `split_vcs_by_assert` and small `assert` proof hints to expose an
   intermediate fact or separate cases.
3. Replace hostile unbounded quantifiers with equivalent frames, bounded
   relations, or recursive helpers. Add valid triggers when quantification is
   unavoidable.
4. Prefer additive recurrences to nonlinear closed forms. Do not wrap built-in
   arithmetic in a helper merely to obscure it.
5. Prove reusable facts as lemmas and instantiate them explicitly with `apply`.
   Put `[weight = N]` on a recursive helper or a `forall ... apply` that the
   analysis names, so the solver stops unrolling or instantiating it on its
   own.
6. Increase the per-condition timeout only after improving the proof shape, up
   to {{ args.max_verification_timeout }}.

Data invariants and global update invariants may help when they express genuine
properties preserved by **every** constructor or mutator. They create new proof
obligations across the module, so do not add them as local solver hints without
checking that global semantic commitment.

{% endif %}
