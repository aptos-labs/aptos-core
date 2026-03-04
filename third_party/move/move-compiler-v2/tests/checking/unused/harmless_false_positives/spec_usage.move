module 0x42::m {
    // Items used only in spec blocks - harmless false positives
    // The warning is harmless because items ARE used (just in specs, not runtime code)

    fun spec_helper(): u64 {
        42
    }

    const SPEC_CONST: u64 = 100;

    struct SpecStruct has key {
        x: u64
    }

    public fun do_something(): u64 {
        1
    }

    spec do_something {
        ensures result == spec_helper() - 41;
        ensures result < SPEC_CONST;
    }

    spec module {
        invariant exists<SpecStruct>(@0x42) ==> global<SpecStruct>(@0x42).x > 0;
    }

    // ========================================
    // Data invariants (struct invariants)
    // ========================================
    const INVARIANT_BOUND: u64 = 1000;

    struct InvariantStruct has drop {
        value: u64
    }

    spec InvariantStruct {
        invariant value < INVARIANT_BOUND;
    }

    public fun make_invariant_struct(): InvariantStruct {
        InvariantStruct { value: 10 }
    }

    // ========================================
    // Spec functions
    // ========================================
    const SPEC_FUN_CONST: u64 = 50;

    struct SpecFunStruct has drop {
        data: u64
    }

    // Function used only inside a spec function
    fun helper_for_spec(): u64 {
        999
    }

    spec fun spec_compute(x: u64): u64 {
        x + SPEC_FUN_CONST + helper_for_spec()
    }

    spec fun spec_validate(s: SpecFunStruct): bool {
        s.data > 0
    }

    public fun validated_operation(x: u64): u64 {
        x * 2
    }

    spec validated_operation {
        ensures result == spec_compute(x) + x - SPEC_FUN_CONST;
    }

    public fun make_spec_fun_struct(): SpecFunStruct {
        SpecFunStruct { data: 5 }
    }

    spec make_spec_fun_struct {
        ensures spec_validate(result);
    }
}
