{# Shared spec writing/editing guidance #}
{% if once(name="move_editing_ref") %}

{% include "templates/move_lang.md" %}
{% include "templates/move_package.md" %}
{% include "templates/core_tools.md" %}

## Writing and Editing Move Code

### Edit–Compile Cycle

When fixing compilation errors, follow this iterative loop:

1. Call `{{ tool(name="move_package_status") }}` with the package path.
2. If the package compiles cleanly, report success and stop.
3. If there are errors, read the diagnostics carefully, and discuss fixes with the user. Then go back to step 1.

{% endif %}
