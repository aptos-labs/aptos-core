module 0xc0ffee::m {
    fun foo(): bool {
        true
    }

    fun bar() {}

    enum Blah {
        Baz(u64),
        Qux(u64),
    }

    fun blah(): Blah {
        Blah::Baz(42)
    }

    /****** Cases with warnings *****/

    public fun test_warn_1() {
        if ({let x = foo(); !x}) {
            bar();
        }
    }

    public fun test_warn_2(x: bool) {
        if ({x = x && foo(); x}) {
            bar();
        }
    }

    public fun test_warn_3() {
        match ({let x = blah(); x}) {
            Blah::Baz(_) => bar(),
            Blah::Qux(_) => {},
        }
    }

    public fun test_warn_4() {
        if ({let x = foo(); x}) {
            if ({let x = foo(); x}) {
                bar();
            }
        } else {
            if ({let x = foo(); x}) {
                bar();
            }
        }
    }

    public fun test_warn_5() {
        // Only warn on the outermost condition.
        if ({if ({let x = foo(); x}) {bar();}; let x = foo(); x}) {
            bar();
        }
    }

    public fun test_warn_6(x: u64) {
        while ({x = x + 1; x < 10}) {
            bar();
        }
    }

    public fun test_warn_7(x: u64) {
        if ({x = x + 1; x < 10} && {x = x + 1; x < 11}) {
            bar();
        }
    }

    /****** Cases without warnings *****/

    public fun test_no_warn_1(x: u64) {
        while ({x = x + 1; spec { invariant x <= 100; }; x < 10}) {
            bar();
        }
    }

    public fun test_no_warn_2(x: u64) {
        if ((x + 1) * (x + 4) > 5) {
            bar();
        }
    }

    public fun test_no_warn_3(x: u64) {
        if ({x = x + 1; x < 10} && {spec { invariant x <= 100; }; x < 10}) {
            bar();
        }
    }
}

#[lint::skip(blocks_in_conditions)]
module 0xc0ffee::no_warn_1 {
    fun foo(): bool {
        true
    }

    fun bar() {}

    enum Blah {
        Baz(u64),
        Qux(u64),
    }

    fun blah(): Blah {
        Blah::Baz(42)
    }

    /****** Suppress warnings *****/

    public fun test_warn_1() {
        if ({let x = foo(); !x}) {
            bar();
        }
    }

    #[lint::skip(blocks_in_conditions)]
    public fun test_warn_2(x: bool) {
        if ({x = x && foo(); x}) {
            bar();
        }
    }

    public fun test_warn_3() {
        match ({let x = blah(); x}) {
            Blah::Baz(_) => bar(),
            Blah::Qux(_) => {},
        }
    }
}

module 0xc0ffee::no_warn_2 {
    fun foo(): bool {
        true
    }

    fun bar() {}

    /****** Suppress warnings *****/

    #[lint::skip(blocks_in_conditions)]
    public fun test_warn_1() {
        if ({let x = foo(); !x}) {
            bar();
        }
    }
}
