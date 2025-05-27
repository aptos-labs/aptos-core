module 0xc0ffee::m {

    public fun test_absorption_law(a: bool, b: bool) {
        // Pattern: a && b || a should simplify to just a
        if ((a) && b || (b)) ();

        // Pattern: a || b && a should simplify to just a
        if ((a) || (b && a)) ();

        // Pattern: a || b && a should simplify to just a
        if ((a) || (b && a)) ();
    }

    public fun test_idempotence(a: bool) {
        // Pattern: a && a should simplify to just a
        if ((a) && (a)) ();

        // Pattern: a || a should simplify to just a
        if ((a) || (a)) ();
    }

    public fun test_identity(a: bool) {
        // Pattern: a && true should simplify to just a
        if ((a) && (true)) ();

        // Pattern: true && a should simplify to just a
        if ((true) && (a)) ();
    }

    public fun test_annihilation(a: bool) {
        // Pattern: a && false should simplify to just false
        if ((a) && (false)) ();

        // Pattern: false && a should simplify to just false
        if ((false) && (a)) ();

        // Pattern: a || true should simplify to just true
        if ((a) || (true)) ();

        // Pattern: true || a should simplify to just true
        if ((true) || (a)) ();
    }

    public fun test_contradiction_tautology(a: bool) {
        // Pattern: a && !a should simplify to just false
        if ((a) && (!a)) ();

        // Pattern: !a && a should simplify to just false
        if ((!a) && (a)) ();

        // Pattern: a || !a should simplify to just true
        if ((a) || (!a)) ();

        // Pattern: !a || a should simplify to just true
        if ((!a) || (a)) ();
    }
}
