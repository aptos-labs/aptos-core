{% if tactic_selectable %}
{{ frontmatter(name="move-inf", description="Infer and verify complete Move specifications and loop invariants. Use when contracts are missing; not merely to check existing specs.", argument_hint="[hybrid-guided|hybrid-flexible] [scope]") }}
{% else %}
{{ frontmatter(name="move-inf", description="Infer and verify complete Move specifications and loop invariants. Use when contracts are missing; not merely to check existing specs.") }}
{% endif %}

{% include "templates/spec_inf_tasks.md" %}

Use the reference sections below as their issues arise.

{% include "templates/spec_inf_ref.md" %}
{% include "templates/verification_ref.md" %}
{% include "templates/spec_inf_report.md" %}
