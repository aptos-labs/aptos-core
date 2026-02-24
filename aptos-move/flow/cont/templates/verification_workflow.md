{% if once(name="verification_workflow") %}

{% include "templates/spec_editing_workflow.md" %}

## Verification 

### Verification Tool

Use `{{ tool(name="move_package_verify") }}` to run the Move Prover on a package and
formally verify its specifications:

- Call with `package_path` set to the package directory and `timeout` set to
  {{ args.initial_verification_timeout }}.
- The tool returns "verification succeeded" when all specs hold, or a diagnostic with a
  counterexample when a spec fails.

#### Narrowing scope with filters

Use the `filter` parameter to restrict the verification scope:

- **Single function:** set `filter` to `module_name::function_name`.
- **Single module:** set `filter` to `module_name`.

#### Excluding targets

Use the `exclude` parameter to skip specific functions or modules while
verifying the rest of the scope:

- **Exclude function(s):** set `exclude` to `["module_name::function_name"]`.
- **Exclude module(s):** set `exclude` to `["module_name"]`.

Exclusions take precedence over the `filter` scope — a target that matches both
`filter` and `exclude` is excluded. This is useful in Phase 2 to skip timed-out
functions without modifying source files.

#### Setting timeout

Verification can be long-running (10 seconds or more). Always explicitly specify a timeout. 
Start with a low timeout of {{ args.initial_verification_timeout }} to get quick feedback.
Increase the timeout to not more than {{ args.max_verification_timeout }} in the case of 
investigating difficult verification problems. 

### Diagnosing Verification Failures

When the prover reports a counterexample or error:

- **Postcondition failure**: The `ensures` clause doesn't hold for some execution path.
  Check whether an edge case is missing or the condition is too strong.
- **Abort condition failure**: An abort path is not covered by `aborts_if`. Trace which
  operations can abort (arithmetic overflow, missing resource, vector out-of-bounds) and
  add the missing condition.
- **Wrong `old()` usage**: Using `old()` in `aborts_if` or `requires` causes a compilation
  error. Remove it — those clauses are already evaluated in the pre-state.
- **Loop-related failures**: Missing or too-weak loop invariants cause havoced variables.
  Strengthen the invariant to constrain all loop-modified variables.
- **Timeout ("out of resources")**:

  > **HARD RULE — do NOT delete, comment out, or weaken any `aborts_if` or
  > `ensures` condition to resolve a timeout.** This includes adding
  > `pragma aborts_if_is_partial;`, which silently suppresses uncovered abort
  > paths. Every condition is assumed semantically correct; removing one hides
  > real properties and makes the specification unsound. If you are tempted to
  > remove a condition because verification is slow, you MUST instead rewrite
  > it in a semantically equivalent form or add axioms/lemmas.

  Timeout resolution strategies (all preserve existing conditions):
  - Split complex `ensures` into multiple simpler clauses.
  - Replace quantifiers with concrete bounds.
  - Add helper lemma functions that break a proof into smaller steps.
  - Add explicit axioms to guide the solver. When you add an axiom, ensure quantifiers have 
    valid triggers. Move quantifiers support a disjunction of a conjunction of triggers.
  - Restructure expressions while preserving their meaning (e.g. factor out common
    sub-expressions into `let` bindings, reorder conjuncts).
  - Document every new helper or axiom with a `///` doc comment explaining
    what property it captures and why it is needed.

  **Avoid non-linear arithmetic in spec helpers.** SMT solvers handle linear
  arithmetic well but struggle with multiplication, division, or modulo between
  two non-constant expressions. When defining helper functions or axioms, prefer
  additive recurrences over closed-form formulas that involve products of
  variables. For example, use `sum_up_to(n) == sum_up_to(n - 1) + n` (linear)
  rather than the closed form `n * (n + 1) / 2` (non-linear). If a non-linear
  closed form is needed for the final specification, connect it to the recursive
  helper via a separate lemma or axiom so the solver can reason about each step
  linearly.

  **Do not redefine built-in operations as spec helpers.** The SMT solver
  already understands arithmetic operators (`*`, `/`, `%`), comparisons, and
  bitwise operations natively. Wrapping them in a recursive spec function
  (e.g. `spec fun mul(a: u64, b: u64): u64 { if (b == 0) { 0 } else { a + mul(a, b - 1) } }`)
  adds an unnecessary unfolding layer that makes solving harder, not easier.
  Only introduce a spec helper when it encodes logic the solver does not have
  built in — such as a loop accumulation pattern or a recursive
  data-structure traversal.

  **Document every helper and axiom.** When introducing a spec helper function
  or axiom, add a `///` doc comment explaining what property it captures and
  why it is needed (e.g. which loop or timeout it supports). Place new spec
  helper functions below the Move function and spec block that introduce them.
  Place axioms for a helper function directly beneath that helper's
  declaration.

### Verification Workflow

Follow this three-phase approach to resolve verification failures efficiently.

**Phase 1 — Full-scope run.** Run verification for the full requested scope with
`timeout` set to {{ args.initial_verification_timeout }}. This gives an overview of all failures —
both logical errors and timeouts. 

**Phase 2 — If they are any logical errors, iterate to fix them using the `exclude` 
parameter of the verify tool to exclude functions whose verification timed out. Only 
continue to phase 3 once all non-timeouts cleanly pass.

**Phase 3 — Resolve timeouts one by one calling prover with a **function-level filter** (see above) 
and apply the timeout resolution strategies described above. As a timeout value,
{{ args.max_verification_timeout }} must be used. If a function cannot be resolved after
{{ args.default_verification_attempts }} attempts and the user did not request otherwise, add
`pragma verify = false;` and keep the specifications so the user can investigate.

**Phase 4 -- Finally run the prover for the full requested scope using as timeout
{{ args.max_verification_timeout }} to verify success.

{% endif %}
