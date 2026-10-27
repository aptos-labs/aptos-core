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
    spec tuple_projection_in_modifies() {
        pragma opaque = true;
        modifies R[{
            let (a_0,a_1,a_2) = result_of<multi>();
            a_0
        }];
        ensures [inferred] ({
            let (a_0,a_1,a_2) = ..S1 |~ result_of<multi>();
            R[a_0].value == a_1 + a_2
        });
        aborts_if [inferred] aborts_of<multi>();
        aborts_if [inferred] ({
            let (a_0,a_1,a_2) = ..S1 |~ result_of<multi>();
            S1 |~ !exists<R>(a_0)
        });
        aborts_if [inferred] ({
            let (a_0,a_1,a_2) = ..S1 |~ result_of<multi>();
            a_1 + a_2 > MAX_U64
        });
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
    spec nested_label_tuple_projection(counter: &mut u64): u64 {
        pragma opaque = true;
        ensures [inferred] ({
            let (a_0,a_1) = result_of<pair_at>(result_of<opaque_address>(old(counter)));
            result == a_0
        });
        ensures [inferred] ensures_of<opaque_address>(old(counter), result_of<opaque_address>(old(counter)), counter);
        aborts_if [inferred] aborts_of<pair_at>(result_of<opaque_address>(counter));
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
    spec nine_hoisted_calls(counter: &mut u64): Nine {
        pragma opaque = true;
        ensures [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64, x4: u64, x5: u64: ensures_of<make>(old(counter), result_of<make>(old(counter)), x) && ensures_of<make>(x, result_of<make>(x), y) && ensures_of<make>(y, result_of<make>(y), z) && ensures_of<make>(z, result_of<make>(z), x1) && ensures_of<make>(x1, result_of<make>(x1), x2) && ensures_of<make>(x2, result_of<make>(x2), x3) && ensures_of<make>(x3, result_of<make>(x3), x4) && ensures_of<make>(x4, result_of<make>(x4), x5) && result == Nine{h1: result_of<make>(old(counter)), h2: result_of<make>(x), h3: result_of<make>(y), h4: result_of<make>(z), h5: result_of<make>(x1), h6: result_of<make>(x2), h7: result_of<make>(x3), h8: result_of<make>(x4), h9: result_of<make>(x5)};
        ensures [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64, x4: u64, x5: u64: ensures_of<make>(old(counter), result_of<make>(old(counter)), x5) && ensures_of<make>(x5, result_of<make>(x5), x4) && ensures_of<make>(x4, result_of<make>(x4), x3) && ensures_of<make>(x3, result_of<make>(x3), x2) && ensures_of<make>(x2, result_of<make>(x2), x1) && ensures_of<make>(x1, result_of<make>(x1), z) && ensures_of<make>(z, result_of<make>(z), y) && ensures_of<make>(y, result_of<make>(y), x) && ensures_of<make>(x, result_of<make>(x), counter);
        aborts_if [inferred] aborts_of<make>(counter);
        aborts_if [inferred = sathard] exists x: u64: ensures_of<make>(counter, result_of<make>(counter), x) && aborts_of<make>(x);
        aborts_if [inferred = sathard] exists x: u64, y: u64: ensures_of<make>(counter, result_of<make>(counter), y) && ensures_of<make>(y, result_of<make>(y), x) && aborts_of<make>(x);
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64: ensures_of<make>(counter, result_of<make>(counter), z) && ensures_of<make>(z, result_of<make>(z), y) && ensures_of<make>(y, result_of<make>(y), x) && aborts_of<make>(x);
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64: ensures_of<make>(counter, result_of<make>(counter), x1) && ensures_of<make>(x1, result_of<make>(x1), z) && ensures_of<make>(z, result_of<make>(z), y) && ensures_of<make>(y, result_of<make>(y), x) && aborts_of<make>(x);
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64: ensures_of<make>(counter, result_of<make>(counter), x2) && ensures_of<make>(x2, result_of<make>(x2), x1) && ensures_of<make>(x1, result_of<make>(x1), z) && ensures_of<make>(z, result_of<make>(z), y) && ensures_of<make>(y, result_of<make>(y), x) && aborts_of<make>(x);
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64: ensures_of<make>(counter, result_of<make>(counter), x3) && ensures_of<make>(x3, result_of<make>(x3), x2) && ensures_of<make>(x2, result_of<make>(x2), x1) && ensures_of<make>(x1, result_of<make>(x1), z) && ensures_of<make>(z, result_of<make>(z), y) && ensures_of<make>(y, result_of<make>(y), x) && aborts_of<make>(x);
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64, x4: u64: ensures_of<make>(counter, result_of<make>(counter), x4) && ensures_of<make>(x4, result_of<make>(x4), x3) && ensures_of<make>(x3, result_of<make>(x3), x2) && ensures_of<make>(x2, result_of<make>(x2), x1) && ensures_of<make>(x1, result_of<make>(x1), z) && ensures_of<make>(z, result_of<make>(z), y) && ensures_of<make>(y, result_of<make>(y), x) && aborts_of<make>(x);
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64, x4: u64, x5: u64: ensures_of<make>(counter, result_of<make>(counter), x5) && ensures_of<make>(x5, result_of<make>(x5), x4) && ensures_of<make>(x4, result_of<make>(x4), x3) && ensures_of<make>(x3, result_of<make>(x3), x2) && ensures_of<make>(x2, result_of<make>(x2), x1) && ensures_of<make>(x1, result_of<make>(x1), z) && ensures_of<make>(z, result_of<make>(z), y) && ensures_of<make>(y, result_of<make>(y), x) && aborts_of<make>(x);
    }

}
/*
Verification: Succeeded.
*/
