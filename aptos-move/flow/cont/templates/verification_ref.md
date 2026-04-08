{% if once(name="verification_ref") %}

{% include "templates/spec_editing_ref.md" %}
{% include "templates/spec_lang_proofs.md" %}

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
`filter` and `exclude` is excluded. This is useful in the "Fix logical errors" task to skip timed-out
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

  Do not delete, comment out, or weaken any `aborts_if` or `ensures`
  condition to resolve a timeout. This includes adding
  `pragma aborts_if_is_partial;`, which silently suppresses uncovered abort
  paths. Every condition is assumed semantically correct; removing one hides
  real properties and makes the specification unsound.

  Timeout resolution strategies — try these in order, and iterate
  aggressively before resorting to `pragma verify_duration_estimate`:

  1. **Add data invariants and global update invariants** to constrain
     resource state. These are checked once per modifying function and then
     assumed at every call site (including inside loops), giving the prover
     facts for free without recursive helpers. See the inference reference
     for details on when to use each kind.

  2. **Introduce spec helper functions** that capture intermediate properties.
     Factor complex `ensures` into compositions of simpler helpers. Each
     helper should express one logical step the solver can verify independently.

  3. **Add lemmas** to establish properties about spec helpers
     (e.g. monotonicity, induction steps) that the solver cannot discover
     on its own. Lemmas are proven propositions — do not introduce axioms.

  4. **Add `proof { ... }` blocks** to function specs or lemmas to guide
     the verifier with `assert`, `apply`, and `calc` steps. Use `apply`
     to instantiate lemmas at specific points in the proof.

  5. **Rewrite spec expressions** while preserving their meaning — factor
     out common sub-expressions into `let` bindings, reorder conjuncts,
     or replace a complex closed-form with a recursive helper connected
     by a lemma.

  When you use universal lemma application, always add triggers, as
  in `forall x: u64 {f(x)} apply lemma_for_f(x)`.

  **Avoid non-linear arithmetic in spec helpers.** SMT solvers handle linear
  arithmetic well but struggle with multiplication, division, or modulo between
  two non-constant expressions. Prefer additive recurrences over closed-form
  products. If a non-linear closed form is needed, connect it to a recursive
  helper via a lemma so the solver reasons about each step linearly.

  **Do not redefine built-in operations as spec helpers.** The SMT solver
  already understands `*`, `/`, `%`, comparisons, and bitwise operations
  natively. Only introduce a spec helper when it encodes logic the solver
  does not have built in — such as a loop accumulation pattern or a
  recursive data-structure traversal.

  **Document every function and lemma.** Add a `///` doc comment explaining
  what property it captures and why it is needed. Place new spec helper
  functions below the Move function that introduces them. Place lemmas
  directly beneath their helper's declaration.

{% endif %}
