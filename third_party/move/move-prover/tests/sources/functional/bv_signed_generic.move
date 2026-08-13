// exclude_for: cvc5
// Regression: number-operation slots of generic parameters, returns and
// struct fields are shared across type instantiations, so a `Bitwise`
// classification acquired through an unsigned instantiation leaks into
// signed instantiations of the same generic. Signed integers have no
// bitvector rendering and must render as `int` even when classified
// `Bitwise`; the comparison arm of the backend used to hit `unreachable!()`.
module 0x42::bv_signed_generic {

    fun id<T>(x: T): T {
        x
    }

    // Seeds `Bitwise` into `id`'s shared parameter and return slots via an
    // unsigned instantiation.
    fun seed(x: u8): u8 {
        id(x & 1)
    }
    spec seed {
        aborts_if false;
        ensures result == (x & 1);
    }

    // Signed instantiation of the same generic: comparing the infected value
    // used to panic the backend.
    fun cmp_signed(a: i8): bool {
        let b = id(a);
        b > 0
    }
    spec cmp_signed {
        aborts_if false;
        ensures result == (a > 0);
    }

    // Same infection through a generic struct field slot.
    struct Box<T> has drop { v: T }

    fun seed_field(x: u8): u8 {
        let b = Box { v: x & 1 };
        b.v
    }
    spec seed_field {
        aborts_if false;
        ensures result == (x & 1);
    }

    fun cmp_field(a: i64): bool {
        let b = Box { v: a };
        b.v < 0
    }
    spec cmp_field {
        aborts_if false;
        ensures result == (a < 0);
    }

    // Negation, arithmetic and equality on infected signed values exercise
    // the sibling rendering paths (declarations, `$Negate`/`$Add` dispatch,
    // equality suffixes).
    fun neg_signed(a: i64): i64 {
        let b = id(a);
        -b
    }
    spec neg_signed {
        aborts_if a == MIN_I64;
        ensures result == -a;
    }

    fun add_signed(a: i64): i64 {
        let b = id(a);
        b + 1
    }
    spec add_signed {
        aborts_if a + 1 > MAX_I64;
        ensures result == a + 1;
    }

    fun eq_signed(a: i32): bool {
        let b = id(a);
        b == 0
    }
    spec eq_signed {
        aborts_if false;
        ensures result == (a == 0);
    }

    // An int-rendered (clamped signed) source cast into a bv-classified
    // unsigned destination needs int->bv marshaling, in code and in specs.
    fun cast_infected(a: i64): u8 {
        let b = id(a);
        (b as u8)
    }
    spec cast_infected {
        aborts_if a < 0 || a > 255;
        ensures result == (a as u8);
    }

    // Spec functions share one number-operation slot across instantiations;
    // declaration and call sites must agree on the (clamped) name for signed
    // instantiations.
    spec module {
        fun sid<T>(x: T): T { x }
    }

    fun wrap<T: drop>(x: T): T {
        x
    }
    spec wrap {
        ensures result == sid(x);
    }

    fun wrap_seed(y: u8): u8 {
        wrap(y & 1)
    }

    fun wrap_signed(a: i64): i64 {
        wrap(a)
    }
}

// Intrinsic maps declare their type parameters as phantom, so the signedness
// clamp must inspect their instantiations directly: an infected signed table
// value marks the whole table Bitwise, and `Table<u8, i8>` must still render
// its value type as `int`.
module 0x42::bv_signed_table {
    use extensions::table::{Self, Table};

    fun id<T>(x: T): T {
        x
    }

    fun seed(x: u8): u8 {
        id(x & 1)
    }
    spec seed {
        aborts_if false;
    }

    fun put(t: &mut Table<u8, i8>, a: i8) {
        table::add(t, 1, id(a));
    }
}

// `int2bv`/`bv2int` conversion wrappers follow the same instantiation-aware
// rendering as their operands: for a signed instantiation both sides render
// as `int` and the conversions are identities (a raw `Bitwise` slot test
// emitted a conversion whose numeric base came from an uninstantiated type
// parameter — invalid Boogie).
module 0x42::bv_conv_roundtrip {
    fun id<T>(x: T): T {
        x
    }

    fun seed(x: u8): u8 {
        id(x & 1)
    }
    spec seed {
        aborts_if false;
    }

    spec fun roundtrip<T>(x: T): T {
        int2bv(bv2int(x))
    }

    fun victim(a: i8): i8 {
        id(a)
    }
    spec victim {
        ensures result == roundtrip(a);
    }
}

// A signed KEY does not clamp the map: keys encode to int regardless of the
// map's rendering, so only the value type participates in bv twin selection.
// `Table<i8, u8>` with a bitwise value must render the bv twin consistently
// with its bv-rendered value operands.
module 0x42::bv_signed_key_table {
    use extensions::table::{Self, Table};

    fun put(t: &mut Table<i8, u8>, a: i8, x: u8) {
        table::add(t, a, x & 1);
    }
}
