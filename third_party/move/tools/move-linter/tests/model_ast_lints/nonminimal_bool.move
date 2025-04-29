module 0xc0ffee::m {

    public fun test_warn_and(x : bool) {
        if (true && x) ();
        if (x && true) ();
        if (false && x) ();
        if (x && false) ();
    }

    public fun test_warn_or(x : bool) {
        if (true || x) ();
        if (x || true) ();
        if (false || x) ();
        if (x || false) ();
    }

    public fun test_warn_iff(x : bool) {
        spec {
            assert x <==> true;
            assert true <==> x;
            assert x <==> false;
            assert false <==> x;
        }
    }

    public fun test_warn_implies(x : bool) {
        spec {
            assert x ==> true;
            assert true ==> x;
            assert x ==> false;
            assert false ==> x;
        }
    }

    public fun test_warn_not() {
        if (!true) ();
        if (!false) ();
    }


    public fun combo() {
        if (!true && false || true) ();
    }

    #[lint::skip(nonminimal_bool)]
    fun test_no_warn(): bool {
        !true
    }
}
