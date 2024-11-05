module 0xc0ffee::m {
    fun foo(): bool {
        true
    }

    public fun test1_warn(): bool {
        if (foo()) true else false
    }

    public fun test2_warn(): bool {
        if (foo()) { false }
        else {
            // because la blah blah
            true
        }
    }

    public fun test3_warn(): bool {
        if (foo()) {
            return true
        } else {
            return false
        }
    }

    public fun test4_warn(x: bool): bool {
        if (x) {
            if (foo()) {
                return false
            } else {
                return true
            }
        };
        x
    }

    public fun test5_warn(x: bool): bool {
        if (x) { return false } else { return false }
    }

    public fun test6_no_warn(): bool {
        if (foo()) {
            return true
        } else { false }
    }

    public fun test7_no_warn(): bool {
        let x = if (foo()) {
            return true
        } else { false };
        !x
    }

    #[lint::skip(needless_bool)]
    public fun test1_no_warn(): bool {
        if (foo()) true else false
    }
}
