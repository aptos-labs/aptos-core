module 0x42::m {
    // Used only in spec - should NOT be marked as unused because specs are part of the module
    struct UsedOnlyInSpec {
        x: u64
    }

    // Used in phantom type parameter
    struct UsedAsPhantom {
        y: u64
    }

    struct Wrapper<phantom T> {
        value: u64
    }

    // Used in ability constraint
    struct UsedInConstraint has copy, drop {
        z: u64
    }

    // Used in inline function
    struct UsedInInline {
        a: u64
    }

    // Used in lambda/closure context
    struct UsedInLambda has drop {
        b: u64
    }

    // Used via type parameter bound
    struct UsedInBound has drop {
        c: u64
    }

    // Unused struct for comparison
    struct ActuallyUnused {
        d: u64
    }

    public fun use_phantom(): Wrapper<UsedAsPhantom> {
        Wrapper { value: 1 }
    }

    public fun use_with_constraint<T: copy + drop>(x: T): UsedInConstraint {
        UsedInConstraint { z: 2 }
    }

    public inline fun use_inline(): UsedInInline {
        UsedInInline { a: 3 }
    }

    public fun use_in_lambda() {
        let _f = |_x: UsedInLambda| { };
    }

    public fun use_bound<T: drop>(x: UsedInBound): T {
        abort 0
    }

    spec module {
        fun spec_function(s: UsedOnlyInSpec): bool {
            s.x > 0
        }
    }
}
