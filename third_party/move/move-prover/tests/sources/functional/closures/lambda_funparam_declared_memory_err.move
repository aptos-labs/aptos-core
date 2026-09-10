// A lambda whose inferred spec applies behavioral predicates to a captured
// function parameter is rejected when the fun type's declared
// `modifies_of`/`reads_of` footprints touch global memory, even with no
// concrete memory-touching target of that type: the evaluator's memory
// signature is the union over all variants (closures, function parameters,
// fun-typed struct fields).
module 0x42::launder_mem {
    struct Counter has key {
        value: u64,
    }

    fun apply(f: |address| has drop, a: address) {
        f(a)
    }
    spec apply {
        pragma opaque;
        modifies_of<f>(a: address) Counter[a];
        aborts_if aborts_of<f>(a);
        ensures ensures_of<f>(a);
    }

    fun outer(g: |address| has drop + copy, a: address) {
        apply(|x| g(x), a)
    }
    spec outer {
        modifies_of<g>(a: address) Counter[a];
    }
}
