// Copyright © Aptos Foundation
// A two-state behavioral predicate (`result_of<r.f>(addr)`) in an
// `ensures` on a function that takes a struct-field closure requires
// the pre-state memory label to be saved at procedure entry, so that
// the `$result_of` evaluator's `old_*` memory slot refers to the
// function's entry state.
//
// `modifies_of<f> *;` on the struct spec gives the BP evaluator a
// (old, cur) memory pair — the signature shape that requires this
// pre-state save. A `reads_of`-only signature has a single memory
// slot and does not need it.

module 0x42::result_of_old_label {
    struct Source has key, drop {
        value: u64,
    }

    // The callback itself is deliberately non-storable.  It must still be
    // registered as a struct-field function variant so invoking it is related
    // to `result_of<r.f>` below.
    struct Reader has drop {
        f: |address|u64 has copy+drop,
    }
    spec Reader {
        modifies_of<f> *;
        invariant forall a: address: !aborts_of<f>(a);
    }

    #[persistent]
    fun safe_read(addr: address): u64 {
        if (exists<Source>(addr)) { Source[addr].value } else { 0 }
    }
    spec safe_read {
        pragma opaque;
        aborts_if false;
        ensures exists<Source>(addr) ==> result == Source[addr].value;
        ensures !exists<Source>(addr) ==> result == 0;
    }

    public fun use_reader(r: &Reader, addr: address): u64 {
        (r.f)(addr)
    }
    spec use_reader {
        aborts_if false;
        ensures result == result_of<r.f>(addr);
    }
}
