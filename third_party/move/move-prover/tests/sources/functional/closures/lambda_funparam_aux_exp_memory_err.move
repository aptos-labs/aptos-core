// A behavioral predicate over a non-concrete function value appearing ONLY
// in an auxiliary condition expression (the `aborts_if .. with ..` code) is
// rejected like one in the condition itself: the guard scans all condition
// expressions, mirroring `compute_spec_memory_usage`.
module 0x42::launder_aux {
    struct Counter has key {
        value: u64,
    }

    fun apply(f: |address|u64 has drop, a: address): u64 {
        f(a)
    }
    spec apply {
        pragma opaque;
        modifies_of<f>(a: address) Counter[a];
        aborts_if aborts_of<f>(a);
    }

    fun outer(g: |address|u64 has drop + copy, a: address): u64 {
        apply(|x| g(x) spec { aborts_if false with result_of<g>(x); }, a)
    }
    spec outer {
        modifies_of<g>(a: address) Counter[a];
    }
}
