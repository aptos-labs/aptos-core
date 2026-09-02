{# Verification workflow, shared by proving and guided inference #}
{% if once(name="verification_tasks") %}

## Verification workflow

1. **Check and scope.** Confirm that the package compiles and identify the exact
   function/module scope. If the request is diagnostic only, do not edit specs.
2. **Run an initial full-scope proof.** Call the prover at timeout
   {{ args.initial_verification_timeout }} to collect logical failures and
   timeouts without exclusions.
3. **Resolve logical failures first.** When repair is authorized, use
   counterexamples and diagnostics to fix the implementation/specification
   mismatch. Timed-out functions may be temporarily excluded while resolving
   independent logical failures.
4. **Resolve timeouts individually.** Use a function filter, timeout up to
   {{ args.max_verification_timeout }}, and the simplification/proof strategies
   below. `split_vcs_by_assert` is diagnostic help, not proof success by itself.
{% if evaluation_mode %}
   After {{ args.default_verification_attempts }} focused attempts, retain and
   report an unresolved failure. Never disable or skip verification or weaken
   an obligation because the budget ended.
{% else %}
   After {{ args.default_verification_attempts }} focused attempts, report the
   remaining obligation and evidence. Do not introduce a trusted boundary
   unless the user or project policy explicitly authorizes it.
{% endif %}
5. **Run a final full-scope proof.** Use timeout
   {{ args.max_verification_timeout }} with no temporary exclusions.
{% if evaluation_mode %}
   Every requested obligation must be present and verified for success.
{% else %}
   Report verified, unresolved, and explicitly trusted functions separately.
{% endif %}
6. **Check the candidate.** When specifications were written or changed, close
   with `{{ tool(name="move_spec_check") }}` over the same scope:
   it proves once more and, unlike the prover, also rejects a contract that
   weakened itself or left an obligation uncovered.

{% include "templates/candidate_check.md" %}

{% endif %}
