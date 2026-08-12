// Closure equality must quotient ghost state in captured values: the VM
// compares captures by runtime value, so two closures over the same function
// with runtime-identical captures are equal regardless of model-only ghosts.
// Runs under the DEFAULT vector theory — closure equality is datatype
// equality, independent of the vector encoding and of `native_equality`.
module 0x42::ghost_field_closure_equality {
    struct S has copy, drop { x: u64 }
    spec S {
        ghost g: u64;
    }

    fun mk(gv: u64): |u64|u64 has copy + drop {
        let s = S { x: 0 };
        spec {
            update s.g = gv;
        };
        |y| y + s.x
    }

    // Ghost-divergent captures are Move-equal.
    fun cmp_eq(): bool {
        mk(1) == mk(2)
    }
    spec cmp_eq {
        ensures result;
    }

    // ...and must not be provably UNEQUAL — pre-fix this VERIFIED (raw
    // datatype equality distinguished the captured ghost), certifying a
    // runtime-false fact. FAILS.
    fun cmp_neq(): bool {
        mk(1) != mk(2)
    }
    spec cmp_neq {
        ensures result; // FAILS: the closures are Move-equal
    }

    // Nested: the outer closure's only capture is ANOTHER closure, so the
    // ghost sits two levels down (outer capture -> inner capture -> ghost).
    // The ghost detection must resolve function-typed captures through the
    // closure inventory rather than classifying every fun type ghost-free.
    fun wrap(f: |u64|u64 has copy + drop): |bool|bool has copy + drop {
        |b| {
            let _f = f;
            b
        }
    }

    fun cmp_wrap_eq(): bool {
        wrap(mk(1)) == wrap(mk(2))
    }
    spec cmp_wrap_eq {
        ensures result;
    }

    fun cmp_wrap_neq(): bool {
        wrap(mk(1)) != wrap(mk(2))
    }
    spec cmp_wrap_neq {
        ensures result; // FAILS: the closures are Move-equal
    }
}
