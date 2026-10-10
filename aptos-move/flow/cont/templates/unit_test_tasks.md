{# Unit test tasks — placed first in agent prompt before reference material #}
{% if once(name="unit_test_tasks") %}

## Test-generation workflow

1. **Establish a clean baseline.** Run the existing tests with baseline
   coverage enabled. Do not edit production code or existing tests to hide a
   pre-existing failure; report it unless the user also asked for a fix.
2. **Choose behavioral cases.** Read the requested function or module and cover
   meaningful success paths, distinct aborts, branches, and boundary values.
   Use uncovered lines as evidence, not as the sole definition of quality.
3. **Write isolated generated tests.** Put new tests in
   `tests/move_flow/<module>_tests.move`. Do not modify user-authored tests.
4. **Validate and diagnose.** Run the tests. Repair generated test setup or
   expectations when they are wrong. If a correct test exposes a production
   bug, preserve the reproducer under `bugs/` and report it rather than changing
   the intended assertion to pass.
5. **Review coverage and redundancy.** Keep behaviorally distinct cases even
   when they cover the same line. Remove only generated tests that duplicate
   both behavior and coverage.
6. **Report the outcome.** Name the cases added, whether all tests pass, the
   useful coverage gained, and any suspected product defect.

{% endif %}
