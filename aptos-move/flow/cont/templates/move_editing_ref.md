{# Shared spec writing/editing guidance #}
{% if once(name="move_editing_ref") %}

{% include "templates/move_lang.md" %}
{% include "templates/move_package.md" %}
{% include "templates/core_tools.md" %}

## Compiler-diagnostic workflow

1. Run `{{ tool(name="move_package_status") }}` at the package root and separate
   compiler errors from warnings.
2. If the user asked only for a check, report the diagnostics without editing.
3. If the user asked for a fix, trace each error to its source and make the
   smallest in-scope correction. Preserve executable intent; do not silence an
   error by weakening visibility, changing public APIs, or inventing address
   bindings unless that is the requested change.
4. Re-run package status after each coherent set of edits. Finish only when it
   reports no compiler errors, or report the remaining blocker precisely.

{% endif %}
