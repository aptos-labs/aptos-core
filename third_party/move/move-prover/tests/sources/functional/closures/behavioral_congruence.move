// A callback's abort condition must agree on equal Move values. Move equality on a value
// holding a `String` or an `Option` compares contents, while the abort condition is an
// uninterpreted function of its arguments, so a callback invoked with a value only known to
// be *equal* to the one a specification names (rebuilt field by field, or returned by an
// opaque function) must still be matched to that specification's `aborts_of`.
//
// The negative cases pass a genuinely different value and must keep failing.
module 0x42::behavioral_congruence {
    use std::option::{Self, Option};
    use std::string::String;

    struct Info has copy, drop, store {
        n: u64,
        id: Option<String>
    }

    struct Callbacks has copy, drop, store {
        f: |Info| has copy + drop + store
    }

    fun same(x: Info): Info { x }
    spec same {
        pragma opaque;
        aborts_if false;
        ensures result == x;
    }

    fun rebuilt(x: Info): Info { Info { n: x.n, id: x.id } }
    spec rebuilt {
        pragma opaque;
        aborts_if false;
        ensures result.n == x.n;
        ensures result.id == x.id;
    }

    // A callback stored in a struct field.

    fun field_equal(cb: &Callbacks, x: Info) {
        (cb.f)(same(x))
    }
    spec field_equal {
        aborts_if aborts_of<cb.f>(x);
    }

    fun field_fieldwise(cb: &Callbacks, x: Info) {
        (cb.f)(rebuilt(x))
    }
    spec field_fieldwise {
        aborts_if aborts_of<cb.f>(x);
    }

    // A callback passed as a parameter.

    fun param_equal(f: |Info|, x: Info) {
        f(same(x))
    }
    spec param_equal {
        aborts_if aborts_of<f>(x);
    }

    // The callback receives a different number: must fail.
    fun field_bumped(cb: &Callbacks, x: Info) {
        (cb.f)(Info { n: x.n + 1, id: x.id })
    }
    spec field_bumped {
        aborts_if x.n + 1 > MAX_U64;
        aborts_if x.n + 1 <= MAX_U64 && aborts_of<cb.f>(x);
    }

    // The callback receives an empty id where `x` may carry one: must fail.
    fun field_dropped_id(cb: &Callbacks, x: Info) {
        (cb.f)(Info { n: x.n, id: option::none() })
    }
    spec field_dropped_id {
        aborts_if aborts_of<cb.f>(x);
    }

    // A ghost field is excluded from equality but readable by a specification, so a
    // callback's abort condition may differ on Move-equal values: must fail.
    struct Tagged has copy, drop, store {
        n: u64
    }
    spec Tagged {
        ghost g: u64;
    }

    struct TaggedCallbacks has copy, drop, store {
        f: |Tagged| has copy + drop + store
    }

    fun ghost_divergent(_cb: &TaggedCallbacks, _a: Tagged, _b: Tagged) {}
    spec ghost_divergent {
        ensures _a == _b ==> (aborts_of<_cb.f>(_a) <==> aborts_of<_cb.f>(_b));
    }
}
