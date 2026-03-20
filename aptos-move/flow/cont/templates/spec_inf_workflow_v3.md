{# Specification inference workflow v3 #}
{% if once(name="spec_inf_workflow_v3") %}

{% include "templates/spec_editing_workflow.md" %}
{% include "templates/verification_workflow.md" %}

## Spec Inference v3

{% include "templates/spec_inf_rules.md" %}


### Inference Workflow v3

{# v3 skips WP completely and lets the model do everything #}

**Phase 1 — Synthesize specifications.**
Synthesize loop invariants and function specifications for all functions
matching the `filter`. Respect existing user specifications. Define recursive spec 
helper functions as needed. You MUST avoid the Common Pitfalls in Spec Expressions 
described above.

**Phase 2 — Verify functions.** 
Using the verification workflow define above, verify the functions matching the filter.


{% endif %}
