{# Unit test tasks — placed first in agent prompt before reference material #}
{% if once(name="unit_test_tasks") %}

## Test Generation Tasks — Execute In Order

**Task: Check package.** Build the package and establish a test coverage
baseline. Fix any compilation errors or failing tests before proceeding.

**Task: Analyze target code.** Identify uncovered lines and testable behaviors
(success paths, abort conditions, edge cases, potential bugs).

**Task: Generate test module.** Create tests in `tests/move_flow/<module>_tests.move`.

**Task: Validate tests.** Run tests. Fix compilation errors and wrong
assertions. Move tests that expose real bugs to `bugs/`.

**Task: Check coverage improvement.** Verify new tests added coverage.
Generate additional tests for remaining gaps if feasible.

**Task: Minimize tests.** Delete tests that add no new coverage. Keep distinct
scenarios; prefer simpler setup and clearer naming.

**Task: Report.** Summarize tests generated, coverage achieved, and any
potential bugs found.

{% endif %}
