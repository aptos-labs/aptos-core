{# Shared final response for every inference tactic. #}
{% if once(name="spec_inf_report") %}

## Final report

End with at most three compact bullets:

- **Result:** State the contract or invariants added and the final
  verification/acceptance status. If unresolved, name the blocking obligation.
- **Strategy:** Name the approach and key tools used, and why that strategy fit
  the problem.
- **Decision points:** Summarize at most two pivotal choices made during the
  work, each paired with the evidence or result that motivated it. Omit routine
  steps and a turn-by-turn transcript.

{% endif %}
