{# Inference tasks — composable: includes verification tasks inline #}
{% if once(name="spec_inf_tasks") %}

## Inference Tasks — Execute In Order

**Skip test functions.** Do not infer specs for `#[test]` or `#[test_only]`
functions — the WP tool also skips them automatically.

**Task: Synthesize loop invariants.** For every loop lacking an invariant in a
function matching the `filter`, add one marked as `[inferred]`. Define
recursive spec helper functions as needed. Avoid the Common Pitfalls
described in the reference material below.
When using `spec_output: "file"`, add loop invariants directly in the source
(they must stay inside the function body), but place any new spec helper
functions and lemmas in the `.spec.move` file inside a `spec module { }` block.

**Task: Infer weakest preconditions.** With invariants in place, run the WP tool with the `filter`.
Let the WP tool generate the specs — do not write them by hand.

**Task: Simplify inferred specs.** Apply the simplification rules from the
reference material below. Every function must keep both `ensures` and
`aborts_if` conditions — do not drop `aborts_if` just because it is hard
to verify.
When using `spec_output: "file"`, all inferred spec helper functions and
lemmas belong in the `.spec.move` file inside a `spec module { }` block,
and function conditions go in `spec fun_name { }` blocks in the same file.

{% include "templates/verification_tasks.md" %}

{% endif %}
