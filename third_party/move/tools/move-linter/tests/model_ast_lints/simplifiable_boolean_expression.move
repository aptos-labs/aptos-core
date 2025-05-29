module 0xc0ffee::m {

    public fun test_absorption_law(a: bool, b: bool) {
        // Pattern: a && b || a should simplify to just a
        if (a && b || b) ();

        // Pattern: a || b && a should simplify to just a
        if (a || b && a) ();

        // Pattern: a || b && a should simplify to just a
        if (a || b && a) ();
    }

    public fun test_idempotence(a: bool) {
        // Pattern: a && a should simplify to just a
        if (a && a) ();

        // Pattern: a || a should simplify to just a
        if (a || a) ();
    }

    public fun test_contradiction_tautology(a: bool) {
        // Pattern: a && !a should simplify to just false
        if (a && !a) ();

        // Pattern: !a && a should simplify to just false
        if (!a && a) ();

        // Pattern: a || !a should simplify to just true
        if (a || !a) ();

        // Pattern: !a || a should simplify to just true
        if (!a || a) ();
    }

    public fun test_distributive_law(a: bool, b: bool, c: bool) {
        // Pattern: (a && b) || (a && c) should simplify to a && (b || c)
        if ((a && b) || (a && c)) ();

        // Pattern: (a || b) && (a || c) should simplify to a || (b && c)
        if ((a || b) && (a || c)) ();
    }
}
