{{ frontmatter(name="move", description="Develop, review, and debug Aptos Move code. Use for general Move work; use move-test, move-prove, or move-inf for their specialized workflows.") }}

## Working on a Move package

Find the package root (`Move.toml`) and preserve the user's requested scope.
Inspect the manifest, package structure, and compiler diagnostics only as needed.
For implementation changes, make the smallest behaviorally appropriate edit and
recheck the package. Do not change named-address bindings, dependencies, or
unrelated modules merely to make an error disappear.

{% include "templates/move_lang.md" %}
{% include "templates/move_package.md" %}
{% include "templates/core_tools.md" %}
