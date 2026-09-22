{# Only treatment-specific orchestration belongs here; WP semantics live in wp_tool.md. #}
{% if once(name="spec_inf_tasks") %}

## Specification inference

Infer a complete specification for the requested function or module.
Preserve executable behavior and user-written specifications.

{% if tactic_selectable %}
### Tactic

Two hybrid tactics are available; the default is
**{{ inference_tactic | replace(from="_", to="-") }}**. An invocation may select
`/move-inf hybrid-guided` or `/move-inf hybrid-flexible`, followed by the scope.
Invocation arguments: `$ARGUMENTS`. Follow only the selected tactic.
{% endif %}
{% if inference_tactic == "agent_only" %}
### Direct tactic

Derive the contract and loop invariants from the implementation and dependency
contracts. Check one coherent candidate, then refine the rejected parts.
{% endif %}
{% if inference_tactic == "hybrid_flexible" or tactic_selectable %}
### Flexible hybrid tactic{% if tactic_selectable %} (`hybrid-flexible`){% endif %}

`{{ tool(name="move_package_wp") }}` is available as an inference pass. Decide
whether and when to use it alongside direct reasoning and invariant synthesis.
It runs on any scope, loops included. Interpret its result using **WP tool**
below.
{% if not args.no_wp_simplification %}
Once the repairable warnings are resolved, simplify as much as the contract
needs while preserving its meaning, then check the candidate.
{% else %}
Check the generated clauses directly; change them only to address a diagnostic.
{% endif %}
{% endif %}
{% if inference_tactic == "hybrid_guided" or tactic_selectable %}
### Guided hybrid tactic{% if tactic_selectable %} (`hybrid-guided`){% endif %}

Follow this order:

1. **Run WP over the requested scope**, including loops, with the requested
   output location.
2. **Handle its diagnostics** as described under **WP tool**. Repair missing
   loop invariants one function at a time and rerun WP. Retain inherited callee
   partiality; retrying the caller cannot eliminate it.
{% if not args.no_wp_simplification %}
3. **Simplify what WP derived** while preserving every result, abort, and frame
   obligation. Use the simplification reference below.
{% endif %}
{% if args.no_wp_simplification %}3{% else %}4{% endif %}. **Check the candidate.**{% if args.no_wp_simplification %} Check the generated clauses directly.{% endif %}
   Repair a timeout using the proof guidance. A counterexample to unmodified,
   warning-free WP output is a tool bug; report the failing condition.
{% endif %}

{% include "templates/candidate_check.md" %}
{% endif %}
