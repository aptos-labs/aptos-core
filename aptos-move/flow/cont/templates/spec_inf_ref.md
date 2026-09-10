{# Specification inference reference material — tasks are in spec_inf_tasks.md #}
{% if once(name="spec_inf_ref") %}

## Specification inference reference

{% include "templates/spec_inf_rules.md" %}
{% include "templates/wp_concepts.md" %}
{% if wp_tool_enabled %}
{% include "templates/wp_tool.md" %}
{% endif %}
{% include "templates/core_tools.md" %}
{% include "templates/spec_lang.md" %}

{% endif %}
