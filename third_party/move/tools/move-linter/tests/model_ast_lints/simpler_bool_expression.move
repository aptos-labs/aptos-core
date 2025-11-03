module 0xc0ffee::m {

    const TRUE_CONST: bool = true;
    const FALSE_CONST: bool = false;

    struct BoolFlags has copy, drop {
        a: bool,
        b: bool,
        c: bool,
    }

    struct NestedStruct has copy, drop {
        flags: BoolFlags,
        enabled: bool,
    }

    public fun helper_function(): bool {
        true
    }

    public fun get_bool_flags(): BoolFlags {
        BoolFlags { a: true, b: false, c: true }
    }

    // ===== DOUBLE NEGATION LAW TESTS (should trigger lint) =====
    // Pattern: !!a should simplify to just a

    public fun test_double_negation_parameters(a: bool) {
        if (!!a) ();
    }

    public fun test_double_negation_variables() {
        let a = true;
        let b = false;

        if (!!a) ();
        if (!(!b)) ();
    }

    public fun test_double_negation_struct_field() {
        let flags = get_bool_flags();

        if (!!flags.a) ();
        if (!(!flags.b)) ();
    }

    public fun test_double_negation_nested_struct() {
        let nested = NestedStruct {
            flags: BoolFlags { a: true, b: false, c: true },
            enabled: true
        };

        if (!!nested.flags.a) ();
        if (!(!nested.enabled)) ();
    }

    // ===== ABSORPTION LAW TESTS (should trigger lint) =====
    // Pattern: a && b || a should simplify to just a
    // Pattern: a || a && b should simplify to just a

    public fun test_absorption_law_parameters(a: bool, b: bool) {
        if (a && b || a) ();
        if (a || a && b) ();
    }

    public fun test_absorption_law_variables() {
        let x = true;
        let y = false;

        if (x && y || x) ();
        if (x || x && y) ();
    }

    public fun test_absorption_law_constants() {
        if (TRUE_CONST && FALSE_CONST || TRUE_CONST) ();
        if (FALSE_CONST || FALSE_CONST && TRUE_CONST) ();
    }

    public fun test_absorption_law_struct_field() {
        let flags = get_bool_flags();

        if (flags.a && flags.b || flags.a) ();
        if (flags.a || flags.a && flags.b) ();
    }

    public fun test_absorption_law_nested_struct() {
        let nested = NestedStruct {
            flags: BoolFlags { a: true, b: false, c: true },
            enabled: true
        };

        if (nested.flags.a && nested.enabled || nested.flags.a) ();
        if (nested.flags.a || nested.flags.a && nested.enabled) ();
    }

    // This test should not trigger the lint, since helper_function() might have side effects
    public fun test_absorption_law_mixed() {
        let x = true;
        let p = 20;

        if ((helper_function()) && p > 10 || helper_function()) ();
        if ((x == helper_function()) || x == helper_function() && p > 10) ();
    }

    // ===== IDEMPOTENCE LAW TESTS (should trigger lint) =====
    // Pattern: a && a should simplify to just a
    // Pattern: a || a should simplify to just a

    public fun test_idempotence_parameters(a: bool) {
        if (!!a && !a) ();
        if (a && a) ();
        if (a || a) ();
    }

    public fun test_idempotence_variables() {
        let flag = true;

        if (flag && flag) ();
        if (flag || flag) ();
    }

    // This test should not trigger the lint, since it's already implemented in `nonminimal_bool`
    public fun test_idempotence_constants() {
        if (TRUE_CONST && TRUE_CONST) ();
        if (FALSE_CONST || FALSE_CONST) ();
    }

    public fun test_idempotence_struct_field() {
        let flags = get_bool_flags();

        if (flags.a && flags.a) ();
        if (flags.b || flags.b) ();
    }

    public fun test_idempotence_nested_struct() {
        let nested = NestedStruct {
            flags: BoolFlags { a: true, b: false, c: true },
            enabled: true
        };

        if (nested.flags.a && nested.flags.a) ();
        if (nested.flags.b || nested.flags.b) ();
    }

    // This test should not trigger the lint, since helper_function() might have side effects
    public fun test_idempotence_mixed() {
        let x = true;

        if ((x == helper_function()) && (x == helper_function())) ();
        if ((x == helper_function()) || (x == helper_function())) ();
    }

    // ===== CONTRADICTION TAUTOLOGY TESTS (should trigger lint) =====
    // Pattern: a && !a should simplify to just false
    // Pattern: !a && a should simplify to just false
    // Pattern: a || !a should simplify to just true
    // Pattern: !a || a should simplify to just true

    public fun test_contradiction_tautology_parameters(a: bool) {
        if (a && !a) ();
        if (!a && a) ();
        if (a || !a) ();
        if (!a || a) ();
    }

    public fun test_contradiction_tautology_variables() {
        let condition = true;

        if (condition && !condition) ();
        if (!condition && condition) ();
        if (condition || !condition) ();
        if (!condition || condition) ();
    }

    // This test should not trigger the lint, since it's already implemented in `nonminimal_bool`
    public fun test_contradiction_tautology_constants() {
        if (TRUE_CONST && !TRUE_CONST) ();
        if (!TRUE_CONST && TRUE_CONST) ();
        if (FALSE_CONST || !FALSE_CONST) ();
        if (!FALSE_CONST || FALSE_CONST) ();
    }

    public fun test_contradiction_tautology_struct_field() {
        let flags = get_bool_flags();

        if (flags.a && !flags.a) ();
        if (!flags.a && flags.a) ();
        if (flags.a || !flags.a) ();
        if (!flags.a || flags.a) ();
    }

    public fun test_contradiction_tautology_nested_struct() {
        let nested = NestedStruct {
            flags: BoolFlags { a: true, b: false, c: true },
            enabled: true
        };

        if (nested.flags.a && !nested.flags.a) ();
        if (!nested.flags.a && nested.flags.a) ();
        if (nested.flags.a || !nested.flags.a) ();
        if (!nested.flags.a || nested.flags.a) ();
    }

    // This test should not trigger the lint, since helper_function() might have side effects
    public fun test_contradiction_tautology_mixed() {
        let x = true;

        if ((x == helper_function()) && !(x == helper_function())) ();
        if (!(x == helper_function()) && (x == helper_function())) ();
        if ((x == helper_function()) || !(x == helper_function())) ();
        if (!(x == helper_function()) || (x == helper_function())) ();
    }

    // ===== DISTRIBUTIVE LAW TESTS (should trigger lint) =====
    // Pattern: (a && b) || (a && c) should simplify to a && (b || c)
    // Pattern: (a || b) && (a || c) should simplify to a || (b && c)

    public fun test_distributive_law_parameters(a: bool, b: bool, c: bool) {
        if ((a && b) || (a && c)) ();
        if ((a || b) && (a || c)) ();
    }

    public fun test_distributive_law_variables() {
        let a = true;
        let b = false;
        let c = true;

        if ((a && b) || (a && c)) ();
        if ((a || b) && (a || c)) ();
    }

    public fun test_distributive_law_constants() {
        if ((TRUE_CONST && FALSE_CONST) || (TRUE_CONST && TRUE_CONST)) ();
        if ((FALSE_CONST || FALSE_CONST) && (FALSE_CONST || TRUE_CONST)) ();
    }

    public fun test_distributive_law_struct_field() {
        let flags = get_bool_flags();
        let other = BoolFlags { a: false, b: true, c: false };

        if ((flags.a && other.a) || (flags.a && other.c)) ();
        if ((flags.a || other.a) && (flags.a || other.c)) ();
    }

    public fun test_distributive_law_nested_struct() {
        let nested = NestedStruct {
            flags: BoolFlags { a: true, b: false, c: true },
            enabled: true
        };

        if ((nested.flags.a && nested.enabled) || (nested.flags.a && nested.flags.c)) ();
        if ((nested.flags.a || nested.enabled) && (nested.flags.a || nested.flags.c)) ();
    }

    // This test should not trigger the lint, since helper_function() might have side effects
    public fun test_distributive_law_mixed() {
        let y = 5;

        if ((helper_function() && TRUE_CONST) || (helper_function() && (y > 10))) ();
        if ((helper_function() || FALSE_CONST) && (helper_function() || (y > 10))) ();
    }


    // ===== LINT SKIP TESTS =====

    #[lint::skip(simpler_bool_expression)]
    public fun test_skipped_simplifiable() {
        let a = true;
        let b = false;

        if (a && a) ();
        if (b || !b) ();
        if (a && !a) ();
    }

    // ===== NEGATIVE TESTS (should NOT trigger lint) =====

    public fun test_parameters_no_lint(a: bool, b: bool, c: bool) {
        if (a && b || c) ();
        if (c || a && b) ();
    }

    public fun test_variables_no_lint() {
        let a = true;
        let b = false;

        if (a && b) ();
        if (b || a) ();
    }

    public fun test_constants_no_lint() {
        if (TRUE_CONST && FALSE_CONST) ();
        if (FALSE_CONST || TRUE_CONST) ();
    }

    public fun test_different_struct_fields_no_lint() {
        let flags = BoolFlags { a: true, b: false, c: true };

        if (flags.a && flags.b) ();
        if (flags.a || flags.c) ();
        if (!flags.a && flags.b) ();
        if (flags.b && !flags.c) ();
    }

    public fun test_nested_struct_no_lint() {
        let nested = NestedStruct {
            flags: BoolFlags { a: true, b: false, c: true },
            enabled: true
        };

        if (nested.flags.a && nested.enabled) ();
        if (nested.flags.a || nested.enabled) ();
        if (!nested.flags.a && nested.enabled) ();
        if (nested.flags.a && !nested.enabled) ();
    }

    public fun test_function_call_no_lint() {
        let x = helper_function();
        let y = helper_function();

        if (x && y) ();
        if (x || y) ();
    }

    // ===== THE FOLLOWING TEST DOES NOT TRIGGER THE LINT, BUT IS INCLUDED FOR FUTURE REFERENCE =====
    // Detecting these patterns is not yet supported.

    public fun test_vector_no_lint() {
        let new_vec = vector[true, false];

        if (new_vec[0] && new_vec[0]) ();
        if (new_vec[1] || new_vec[1]) ();
    }
}
