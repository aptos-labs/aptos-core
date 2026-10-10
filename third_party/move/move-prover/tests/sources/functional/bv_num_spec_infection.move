// A bitwise-classified caller argument must not force bitvector rendering
// onto a callee spec's widthless `num` expressions: `num` is an int sink
// (spec lets over shifts, `num`-typed spec fun parameters), and genuinely
// bitvector values convert exactly at the crossing (cast source, spec fun
// argument). One bitwise caller shares the callee's classification slots
// with every clean caller, so both must verify. Mirrors the shape of
// `fixed_point32::create_from_rational`'s abort schema.
module 0x42::bv_num_spec_infection {
    struct Fixed has copy, drop, store {
        value: u64,
    }

    const EDENOM: u64 = 0x10001;
    const ERANGE: u64 = 0x20001;

    public fun create(numerator: u64, denominator: u64): Fixed {
        let _ = denominator;
        Fixed { value: numerator }
    }
    spec create {
        pragma opaque;
        pragma verify = false;
        include CreateAbortsIf;
        ensures result == spec_create(numerator, denominator);
    }
    spec schema CreateAbortsIf {
        numerator: u64;
        denominator: u64;
        let scaled_numerator = (numerator as u128) << 64;
        let scaled_denominator = (denominator as u128) << 32;
        let quotient = scaled_numerator / scaled_denominator;
        aborts_if scaled_denominator == 0 with EDENOM;
        aborts_if quotient == 0 && scaled_numerator != 0 with ERANGE;
        aborts_if quotient > MAX_U64 with ERANGE;
    }
    spec fun spec_create(numerator: num, denominator: num): Fixed {
        Fixed {
            value: ((numerator << 64) / (denominator << 32)) as u64,
        }
    }

    fun mk_clean(n: u64, d: u64): Fixed {
        create(n, d)
    }

    fun mk_bitwise(n: u64): Fixed {
        create(n & 3, 7)
    }
}

// The `num` clamp has containment-closure semantics: a spec-function slot
// that acquired `Bitwise` from one caller can be instantiated at
// `vector<num>` elsewhere; widthless `num` has no bitvector rendering at
// any nesting depth, so that instantiation must render plainly.
module 0x42::bv_num_vector_clamp {
    spec fun accepts<T>(x: T): bool {
        true
    }

    // Aggregate constructors propagate their element classification into
    // the recorded signature.
    spec fun wrap(x: u8): vector<u8> {
        vector[x & 1]
    }

    spec fun bumped<T>(r: &mut u64, x: T): bool {
        old(r) <= r
    }

    // An explicit schema binding substitutes a bitvector-rendered
    // temporary under a schema field typed `num`; spec-fun arguments and
    // casts consult the operand's own rendering at that boundary.
    spec schema BitsBound {
        n: num;
        ensures accepts<num>(n);
    }

    fun mask(r: &mut u64, x: u8, y: u64): u8 {
        *r = *r + 1;
        (x & 1) + ((y & 1) as u8)
    }
    spec mask {
        aborts_if r + 1 > MAX_U64;
        ensures accepts<u8>(x & 1);
        ensures accepts<vector<num>>(vector[MAX_U64]);
        // A generic parameter instantiated at `num` is a sink like a
        // declared `num` parameter: a bitwise argument converts at the
        // boundary, and mixed bitwise/arithmetic uses of the `num`
        // instantiation are not a classification conflict.
        ensures accepts<num>(x & 1);
        ensures accepts<num>(MAX_U64 + 1);
        // The doubled-argument path (`uses_old` with a mutable-reference
        // parameter) converts by-value arguments at the same instantiated
        // boundary.
        ensures bumped<num>(r, x & 1);
        ensures wrap(x) == vector[x & 1];
        // Note: included after the unsigned `accepts` calls above — the
        // schema's `accepts<num>` call must not be the slot-seeding one
        // (known order-dependence of the shared spec-fun slots).
        include BitsBound { n: y };
    }
}
