{# Specification inference workflow v2 #}
{% if once(name="spec_inf_workflow_v2") %}

{% include "templates/spec_editing_workflow.md" %}
{% include "templates/verification_workflow.md" %}

## Spec Inference v2

{% include "templates/wp_tool.md" %}
{% include "templates/spec_inf_rules.md" %}


### Inference Workflow v2

{# v2 doesn't initially run WP but immediately lets the model infer loop invariants #}

**Phase 1 — Synthesize loop invariants.** 
For every loop lacking an invariant in a function matching the `filter`, add 
one marked as `[inferred]`. Define recursive spec helper functions as needed. You
MUST avoid the Common Pitfalls in Spec Expressions described above.

**Phase 2 — Run WP.** With invariants in place, run the WP tool with the `filter`.

**Phase 3 — Simplify and verify functions matching the `filter`.** Simplify the inferred 
specifications described above. Then verify the result using the verification workflow.


{% endif %}
