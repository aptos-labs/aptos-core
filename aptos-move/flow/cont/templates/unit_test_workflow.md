{# Unit test generation workflow #}

## Test Design Rules

These rules govern all test generation. Apply them throughout the workflow below.

**HARD RULE — One behavior per test.** Each test function MUST verify exactly one
scenario. Do not combine success and failure cases, or test multiple edge cases
in a single function. Split complex tests into multiple focused tests.

**HARD RULE — Minimal setup.** Only initialize resources and state required for
the specific behavior being tested. Do not copy boilerplate setup between tests;
each test should set up only what it needs.

**HARD RULE — Test the target function.** Every test MUST call the function
specified by the user. Do not generate tests that only exercise helper functions
or setup code.

### Naming Conventions

- Test function names: `test_<function>_<scenario>` (e.g., `test_transfer_insufficient_balance`)
- Test module names: `<module>_tests` (e.g., `coin_tests`)
- Use descriptive scenario names that explain what is being verified

### Test Structure

Every test follows this pattern:

```move
#[test(account = @0x1)]
fun test_function_scenario(account: &signer) {
    // 1. Setup: Initialize only required resources
    // 2. Action: Call the target function
    // 3. Assert: Verify expected outcomes
}
```

For failure tests, use `#[expected_failure]`:

```move
#[test(account = @0x1)]
#[expected_failure(abort_code = E_INVALID_AMOUNT, location = my_module)]
fun test_function_rejects_invalid_input(account: &signer) {
    // Setup conditions that trigger the abort
    my_module::target_function(account, invalid_input);
}
```

### Common Mistakes to Avoid

- **RESOURCE_ALREADY_EXISTS**: Resources can only be moved to an account once.
  Do not initialize the same resource twice in setup.
- **MISSING_DATA**: Functions that read resources abort if the resource does not
  exist. Ensure required resources are initialized before the action.
- **Signer mismatch**: Operations that check `signer::address_of()` require the
  correct signer. Match signers to expected addresses in test attributes.
- **Import errors**: Use named addresses in imports (e.g., `aptos_framework::coin`),
  not raw addresses.

## Unit Test Generation Workflow

Generate tests by following these phases in order. Use the rules defined above
throughout.

### Phase 1 — Discover and Verify Package

1. Call `{{ tool(name="move_package_manifest") }}` with `package_path` set to
   the package directory to discover source files and dependencies.
2. Call `{{ tool(name="move_package_status") }}` to verify the package compiles.
   If compilation errors are reported, they must be fixed before proceeding.
   Do not generate tests for code that does not compile.

### Phase 2 — Read and Analyze Target Code

1. Read the source file containing the target function.
2. Identify for the target function:
   - **Pre-conditions**: What must be true before calling? (resources exist,
     parameter constraints, access control)
   - **Post-conditions**: What should be true after? (state changes, return
     values, events emitted)
   - **Abort conditions**: What inputs or states cause the function to abort?
     (Look for `assert!`, `abort`, and operations that can fail like resource
     access or arithmetic)
3. List behaviors to test based on the analysis above:
   - One success path with valid inputs
   - One test per distinct abort condition
   - Edge cases if applicable (empty collections, zero values, boundary values)

### Phase 3 — Generate Test Module

Create a test module with one test per behavior identified in Phase 2. Follow
the test structure and naming conventions defined above.

```move
#[test_only]
module <package_address>::<module>_tests {
    use <package_address>::<module>;
    // Imports for test utilities as needed

    // Success test
    #[test(account = @0x1)]
    fun test_<function>_success(account: &signer) {
        // Minimal setup
        // Call target function
        // Assert expected state
    }

    // Failure test for each abort condition
    #[test(account = @0x1)]
    #[expected_failure(abort_code = <CODE>, location = <module>)]
    fun test_<function>_<abort_scenario>(account: &signer) {
        // Setup that triggers abort
        // Call target function (will abort)
    }
}
```

### Phase 4 — Validate Tests

Run tests using:
```bash
aptos move test --package-dir <package_path>
```

If tests fail:

1. **Compilation errors**: Fix syntax, imports, or type mismatches. Re-read
   source if needed to verify correct types and function signatures.
2. **Unexpected abort**: The test triggered an abort that was not expected.
   Either add `#[expected_failure]` if this is the intended behavior, or fix
   the setup to satisfy pre-conditions.
3. **Expected abort did not occur**: The abort condition was not triggered.
   Verify the setup correctly creates the failing condition.
4. **Assertion failed**: The post-condition check failed. Verify the expected
   value matches actual behavior.

Iterate until all tests pass.

### Phase 5 — Coverage Improvement (Optional)

If the user requests coverage analysis, run:
```bash
aptos move test --package-dir <package_path> --coverage
```

Review uncovered code paths and add tests for:
- Uncovered branches (if/else, match arms)
- Untested abort conditions
- Edge cases not yet covered

Apply the same rules and structure defined above for any new tests.
