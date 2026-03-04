{# Specification inference workflow #}
{% if once(name="spec_inf_workflow") %}

{% include "templates/spec_editing_workflow.md" %}
{% include "templates/verification_workflow.md" %}

## Spec Inference

{% include "templates/wp_tool.md" %}
{% include "templates/spec_inf_rules.md" %}

### Inference Workflow

**Phase 1 — Initial WP run.** Run the WP tool on the functions matching the `filter`
to get raw inferred specifications. Loops without invariants will produce
`[inferred = vacuous]` conditions — this is expected.

**Phase 2 — Synthesize loop invariants.** For every loop lacking an invariant
in a function matching the `filter`, add one marked `[inferred]`. Define
recursive spec helper functions as needed. Remove all `[inferred = *]` conditions before 
proceeding. You MUST avoid the Common Pitfalls in Spec Expressions described above.

**Phase 3 — Re-run WP.** Re-run the WP tool on the same scope. With invariants
in place, the tool should now produce complete (non-vacuous) specifications.

**Phase 4 — Simplify and verify.** Simplify the inferred specifications
described above (remove redundancy, strengthen where obvious). Then verify the
result using the verification workflow described above.


{% endif %}
