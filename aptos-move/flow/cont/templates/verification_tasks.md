{# Verification tasks — placed first in agent prompt before reference material #}
{% if once(name="verification_tasks") %}

## Verification Tasks — Execute In Order

**Task: Full-scope verification run.** Run verification for the full requested scope
with `timeout` set to {{ args.initial_verification_timeout }}. This gives an
overview of all failures — both logical errors and timeouts.

**Task: Fix logical errors.** If there are any logical errors, iterate to fix them
using the `exclude` parameter of the verify tool to exclude functions whose
verification timed out. Only continue once all non-timeouts cleanly pass.

**Task: Resolve timeouts.** Resolve timeouts one by one calling the prover with a
function-level filter and `timeout` set to {{ args.max_verification_timeout }}.
Apply the timeout resolution strategies from the reference material below
(spec helpers, lemmas, proofs). If a function cannot be resolved after
{{ args.default_verification_attempts }} attempts and the user did not request
otherwise, add `pragma verify_duration_estimate = N;` where `N` is the exact
timeout at which you observed verification succeed. If verification never
succeeded, use `pragma verify = false;` instead.

**Task: Final full-scope verification.** Run the prover for the full requested scope
using `timeout` {{ args.max_verification_timeout }} to verify success. Functions
with `pragma verify_duration_estimate = N;` where `N` exceeds the timeout will
be automatically skipped — this is expected.

{% endif %}
