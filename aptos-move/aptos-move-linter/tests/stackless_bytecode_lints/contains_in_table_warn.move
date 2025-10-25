module 0x42::table_usage {
    use aptos_std::table;

    struct Table has key, store {
        table: table::Table<u64, u64>,
    }

    // Helper function for testing function calls between contains and operation
    fun some_helper_function() {
        // This function might potentially modify state
    }

    // =================================================================
    // POSITIVE TESTS - These should be SAFE (no warnings)
    // =================================================================

    // Basic contains check before borrow
    public fun test_safe_borrow(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (table::contains(&table.table, 1)) {
            table::borrow(&table.table, 1); // SAFE: key exists
        };

        move_to(account, table);
    }

    // Basic contains check before add (else branch)
    public fun test_safe_add_else(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (table::contains(&table.table, 1)) {
            // Do nothing
        } else {
            table::add(&mut table.table, 1, 100); // SAFE: key doesn't exist
        };

        move_to(account, table);
    }

    // Negated contains before add
    public fun test_safe_negated_add(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (!table::contains(&table.table, 1)) {
            table::add(&mut table.table, 1, 100); // SAFE: key doesn't exist
        };

        move_to(account, table);
    }

    // Multiple operations with different keys
    public fun test_safe_multiple_keys(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (table::contains(&table.table, 1)) {
            table::borrow(&table.table, 1); // SAFE: key 1 exists
        };

        if (!table::contains(&table.table, 2)) {
            table::add(&mut table.table, 2, 200); // SAFE: key 2 doesn't exist
        };

        move_to(account, table);
    }

    // Assignment of contains result
    public fun test_assigned_contains(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        let exists = table::contains(&table.table, 1);
        if (exists) {
            table::borrow(&table.table, 1); // Should be SAFE - linter handles this!
        } else {
            table::add(&mut table.table, 1, 100); // Should be SAFE - linter handles this!
        };

        move_to(account, table);
    }

    // Negated assignment of contains result
    public fun test_negated_assigned_contains(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        let not_exists = !table::contains(&table.table, 1);
        if (not_exists) {
            table::add(&mut table.table, 1, 100); // Should be SAFE
        } else {
            table::borrow(&table.table, 1); // Should be SAFE
        };

        move_to(account, table);
    }

    // Early return pattern
    public fun test_early_return_pattern(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (!table::contains(&table.table, 1)) {
            move_to(account, table);
            return; // Early return
        };

        table::borrow(&table.table, 1); // Should be SAFE - key exists
        move_to(account, table);
    }

    // Use assert
    public fun test_assert(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        assert!(table::contains(&table.table, 1));
        table::borrow(&table.table, 1); // Should be SAFE - key exists
        move_to(account, table);
    }

    // Use negated assert
    public fun test_negated_assert(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        assert!(!table::contains(&table.table, 1));
        table::add(&mut table.table, 1, 100); // Should be SAFE
        move_to(account, table);
    }

    // Double negation
    public fun test_double_negation(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (!!table::contains(&table.table, 1)) {
            table::borrow(&table.table, 1); // Should be SAFE
        };

        move_to(account, table);
    }

    // Negation after assignment
    public fun test_negation_after_assignment(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        let exists = table::contains(&table.table, 1);
        let not_exists = !exists;
        if (not_exists) {
            table::add(&mut table.table, 1, 100); // Should be SAFE
        };

        move_to(account, table);
    }

    // =================================================================
    // NEGATIVE TESTS - These should generate warnings
    // =================================================================

    // Borrow without any contains check
    public fun test_unsafe_borrow_no_check(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        table::borrow(&table.table, 1); // UNSAFE: no contains check

        move_to(account, table);
    }

    // Add without any contains check
    public fun test_unsafe_add_no_check(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        table::add(&mut table.table, 1, 100); // UNSAFE: no contains check

        move_to(account, table);
    }

    // Borrow with wrong key contains check
    public fun test_unsafe_borrow_wrong_key(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (table::contains(&table.table, 1)) {
            table::borrow(&table.table, 2); // UNSAFE: checked key 1, borrowing key 2
        };

        move_to(account, table);
    }

    // Add with wrong key contains check
    public fun test_unsafe_add_wrong_key(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (!table::contains(&table.table, 1)) {
            table::add(&mut table.table, 2, 200); // UNSAFE: checked key 1, adding key 2
        };

        move_to(account, table);
    }

    // Negated contains before borrow
    public fun test_unsafe_negated_borrow(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (!table::contains(&table.table, 1)) {
            table::borrow(&table.table, 1); // UNSAFE: key doesn't exist
        };

        move_to(account, table);
    }

    // Assignment with wrong key
    public fun test_assigned_wrong_key(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        let exists = table::contains(&table.table, 1);
        if (exists) {
            table::borrow(&table.table, 2); // UNSAFE - checked key 1, borrowing key 2
        };

        move_to(account, table);
    }

    // =================================================================
    // SKIP LINT TESTS - These should not generate warnings
    // =================================================================

    #[lint::skip(contains_in_table)]
    public fun test_skip_lint_unsafe_borrow(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        table::borrow(&table.table, 1); // Should be skipped

        move_to(account, table);
    }

    #[lint::skip(contains_in_table)]
    public fun test_skip_lint_unsafe_add(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        table::add(&mut table.table, 1, 100); // Should be skipped

        move_to(account, table);
    }

    // =================================================================
    // EDGE CASES - Where the linter FAILS to detect safe/unsafe operations
    // =================================================================

    // Complex boolean expressions
    public fun test_complex_boolean_and(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };
        let other_condition = true;

        if (table::contains(&table.table, 1) && other_condition) {
            table::borrow(&table.table, 1); // Should be SAFE - linter does not handle this correctly!
        };

        move_to(account, table);
    }

    // Assignment with complex boolean expressions
    public fun test_assigned_boolean_expression(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };
        let other = true;

        let can_borrow = table::contains(&table.table, 1) && other;
        if (can_borrow) {
            table::borrow(&table.table, 1); // Should be SAFE - linter does not handle this correctly!
        };

        move_to(account, table);
    }

    // Multiple assignment levels
    public fun test_multiple_assignment_levels(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        let contains_result = table::contains(&table.table, 1);
        let exists = contains_result;
        let key_found = exists;

        if (key_found) {
            table::borrow(&table.table, 1); // Should be SAFE - linter does not handle this correctly!
        };

        move_to(account, table);
    }

    // Function calls between contains and operation
    public fun test_function_call_between(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        // Currently the linter does not analyze the function call, so even this is flagged as SAFE
        // this use case is not correctly handled.
        if (table::contains(&table.table, 1)) {
            some_helper_function(); // Might invalidate knowledge
            table::borrow(&table.table, 1); // Should be SAFE, but what if helper modifies table?
        };

        move_to(account, table);
    }

    // Reassignment of variable
    public fun test_variable_reassignment(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        let _exists = table::contains(&table.table, 1);
        _exists = false; // Reassign to false
        if (_exists) {
            table::borrow(&table.table, 1); // UNSAFE - exists was reassigned to false but linter does not handle this correctly!
        };

        move_to(account, table);
    }

    // OR expression
    public fun test_or_expression(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };
        let other_condition = false;

        if (table::contains(&table.table, 1) || other_condition) {
            table::borrow(&table.table, 1); // CORRECTLY FLAGGED - unsafe when other_condition=true, linter ignores this!
        };

        move_to(account, table);
    }

    // =================================================================
    // BORDER CASES
    // =================================================================

    // Multiple table instances
    public fun test_multiple_tables(account: &signer) {
        let table1 = Table {
            table: table::new<u64, u64>(),
        };
        let table2 = Table {
            table: table::new<u64, u64>(),
        };

        if (table::contains(&table1.table, 1)) {
            table::borrow(&table1.table, 1); // SAFE: same table
            table::borrow(&table2.table, 1); // UNSAFE: different table
        };

        move_to(account, table1);
        move_to(account, table2);
    }

    // Nested conditions
    public fun test_nested_conditions(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (table::contains(&table.table, 1)) {
            if (table::contains(&table.table, 2)) {
                table::borrow(&table.table, 1); // SAFE: key 1 checked in outer if
                table::borrow(&table.table, 2); // SAFE: key 2 checked in inner if
            };
        };

        move_to(account, table);
    }

    // Multiple operations on same key after contains
    public fun test_multiple_operations_same_key(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (table::contains(&table.table, 1)) {
            table::borrow(&table.table, 1); // SAFE
            table::borrow(&table.table, 1); // SAFE: same key, still checked
        };

        move_to(account, table);
    }

    // Complex control flow
    public fun test_complex_control_flow(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (table::contains(&table.table, 1)) {
            table::borrow(&table.table, 1); // SAFE
        } else {
            table::add(&mut table.table, 1, 100); // SAFE
        };

        if (!table::contains(&table.table, 2)) {
            table::add(&mut table.table, 2, 200); // SAFE
        } else {
            table::borrow(&table.table, 2); // SAFE
        };

        move_to(account, table);
    }

    // Loop scenarios
    public fun test_loop_scenario(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };
        let i = 0;

        while (i < 3) {
            if (table::contains(&table.table, (i as u64))) {
                table::borrow(&table.table, (i as u64)); // Should be SAFE - linter does not handle this correctly!
            };
            i = i + 1;
        };

        move_to(account, table);
    }

    // Table modification between contains and operation
    public fun test_table_modification_between(account: &signer) {
        let table = Table {
            table: table::new<u64, u64>(),
        };

        if (table::contains(&table.table, 1)) {
            table::add(&mut table.table, 2, 200); // Modifying table (but different key) -> This add is not safe
            table::borrow(&table.table, 1); // Should still be SAFE (key 1 unchanged)
        };

        move_to(account, table);
    }
}
