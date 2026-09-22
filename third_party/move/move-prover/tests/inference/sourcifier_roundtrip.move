// Regression coverage for two source-emission bugs found while preparing the
// inference corpus: tuple-valued result_of projections (including projections
// whose base becomes a block during label hoisting) must become destructuring
// lets, and more than six hoisted calls need distinct let names.
module 0x42::sourcifier_roundtrip {
    struct R has key { value: u64 }

    fun multi(): (address, u64, u64) acquires R {
        let value = borrow_global<R>(@0x42).value;
        (@0x42, value, value)
    }

    spec multi {
        pragma opaque = true;
        pragma inference = none;
        aborts_if !exists<R>(@0x42);
        ensures result_1 == @0x42;
        ensures result_2 == global<R>(@0x42).value;
        ensures result_3 == global<R>(@0x42).value;
    }

    fun tuple_projection_in_modifies() acquires R {
        let (address, x, y) = multi();
        borrow_global_mut<R>(address).value = x + y;
    }

    fun opaque_address(counter: &mut u64): address {
        *counter = 0;
        @0x42
    }

    spec opaque_address(counter: &mut u64): address {
        pragma opaque = true;
        pragma inference = none;
        aborts_if false;
        ensures counter == 0;
        ensures result == @0x42;
    }

    fun pair_at(address: address): (u64, u64) acquires R {
        let value = borrow_global<R>(address).value;
        (value, value)
    }

    spec pair_at {
        pragma opaque = true;
        pragma inference = none;
        aborts_if !exists<R>(address);
        ensures result_1 == global<R>(address).value;
        ensures result_2 == global<R>(address).value;
    }

    fun nested_label_tuple_projection(counter: &mut u64): u64 acquires R {
        let address = opaque_address(counter);
        let (first, _) = pair_at(address);
        first
    }

    struct Handle has copy, drop { value: u64 }

    fun make(counter: &mut u64): Handle {
        *counter = *counter + 1;
        Handle { value: *counter }
    }

    spec make {
        pragma opaque = true;
        pragma inference = none;
        aborts_if counter == MAX_U64;
        ensures counter == old(counter) + 1;
        ensures result.value == counter;
    }

    struct Nine has copy, drop {
        h1: Handle,
        h2: Handle,
        h3: Handle,
        h4: Handle,
        h5: Handle,
        h6: Handle,
        h7: Handle,
        h8: Handle,
        h9: Handle,
    }

    fun nine_hoisted_calls(counter: &mut u64): Nine {
        let h1 = make(counter);
        let h2 = make(counter);
        let h3 = make(counter);
        let h4 = make(counter);
        let h5 = make(counter);
        let h6 = make(counter);
        let h7 = make(counter);
        let h8 = make(counter);
        let h9 = make(counter);
        Nine { h1, h2, h3, h4, h5, h6, h7, h8, h9 }
    }
}
