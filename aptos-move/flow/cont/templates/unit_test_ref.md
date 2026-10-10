{# Unit test generation workflow #}
{% if once(name="unit_test_ref") %}

{% include "templates/move_package.md" %}
{% include "templates/core_tools.md" %}

## Unit-test reference

{% include "templates/unit_test_rules.md" %}

### Tools

- Start with `{{ tool(name="move_package_test") }}` and
  `establish_baseline: true`. A failed baseline is pre-existing evidence, not
  permission to edit unrelated code.
- Use `{{ tool(name="move_package_coverage") }}` with
  `function: "module::function"` for a focused target, or omit `function` for
  package-wide uncovered lines.
- After adding tests, call `{{ tool(name="move_package_test") }}` without
  baseline mode. Its `newly_covered` result measures coverage added since the
  baseline.

### Generated test location

Create or extend only `tests/move_flow/<module>_tests.move`:

```move
#[test_only]
module <package_address>::<module>_tests {
    use <package_address>::<module>;

    #[test(account = @0x1)]
    /// @ai-generated
    /// Verifies that <function> <behavior>.
    fun test_<function>_<scenario>(account: &signer) { ... }
}
```

For module-wide work, proceed one module at a time. A suspected-bug test should
assert the intended behavior and therefore remain a faithful failing reproducer
against buggy code.

### Diagnosing a generated test failure

- **Compilation error**: Fix the test
- **Wrong setup or assertion**: Correct the generated test.
- **Likely production defect**: Move the generated reproducer to `bugs/` at
  the package root and document expected versus actual behavior. Do not alter
  production code unless requested.

Only edit generated files under `tests/move_flow/` and `bugs/` during this
workflow.

{% endif %}
