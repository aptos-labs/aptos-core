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
            let (a_0,a_1) = {
                let b = ..S1 |~ result_of<opaque_address>(old(counter));
                S1.. |~ result_of<pair_at>(b)
            };
            result == a_0
        });
        ensures [inferred] ({
            let a = ..S1 |~ result_of<opaque_address>(old(counter));
            ..S1 |~ ensures_of<opaque_address>(old(counter), a, counter)
        });
        aborts_if [inferred] ({
            let a = ..S1 |~ result_of<opaque_address>(counter);
            S1 |~ aborts_of<pair_at>(a)
        });
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
        ensures [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64, x4: u64, x5: u64: {
            let a = ..S1 |~ result_of<make>(old(counter));
            ..S1 |~ ensures_of<make>(old(counter), a, x)
        } && {
            let b = S1..S2 |~ result_of<make>(x);
            S1..S2 |~ ensures_of<make>(x, b, y)
        } && {
            let c = S2..S3 |~ result_of<make>(y);
            S2..S3 |~ ensures_of<make>(y, c, z)
        } && {
            let d = S3..S4 |~ result_of<make>(z);
            S3..S4 |~ ensures_of<make>(z, d, x1)
        } && {
            let e = S4..S5 |~ result_of<make>(x1);
            S4..S5 |~ ensures_of<make>(x1, e, x2)
        } && {
            let f = S5..S6 |~ result_of<make>(x2);
            S5..S6 |~ ensures_of<make>(x2, f, x3)
        } && {
            let a_1 = S6..S7 |~ result_of<make>(x3);
            S6..S7 |~ ensures_of<make>(x3, a_1, x4)
        } && {
            let b_1 = S7..S8 |~ result_of<make>(x4);
            S7..S8 |~ ensures_of<make>(x4, b_1, x5)
        } && result == Nine{h1: ..S1 |~ result_of<make>(old(counter)), h2: S1..S2 |~ result_of<make>(x), h3: S2..S3 |~ result_of<make>(y), h4: S3..S4 |~ result_of<make>(z), h5: S4..S5 |~ result_of<make>(x1), h6: S5..S6 |~ result_of<make>(x2), h7: S6..S7 |~ result_of<make>(x3), h8: S7..S8 |~ result_of<make>(x4), h9: S8.. |~ result_of<make>(x5)};
        ensures [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64, x4: u64, x5: u64: {
            let a = ..S1 |~ result_of<make>(old(counter));
            ..S1 |~ ensures_of<make>(old(counter), a, x5)
        } && {
            let b = S1..S2 |~ result_of<make>(x5);
            S1..S2 |~ ensures_of<make>(x5, b, x4)
        } && {
            let c = S2..S3 |~ result_of<make>(x4);
            S2..S3 |~ ensures_of<make>(x4, c, x3)
        } && {
            let d = S3..S4 |~ result_of<make>(x3);
            S3..S4 |~ ensures_of<make>(x3, d, x2)
        } && {
            let e = S4..S5 |~ result_of<make>(x2);
            S4..S5 |~ ensures_of<make>(x2, e, x1)
        } && {
            let f = S5..S6 |~ result_of<make>(x1);
            S5..S6 |~ ensures_of<make>(x1, f, z)
        } && {
            let a_1 = S6..S7 |~ result_of<make>(z);
            S6..S7 |~ ensures_of<make>(z, a_1, y)
        } && {
            let b_1 = S7..S8 |~ result_of<make>(y);
            S7..S8 |~ ensures_of<make>(y, b_1, x)
        } && {
            let c_1 = S8.. |~ result_of<make>(x);
            S8.. |~ ensures_of<make>(x, c_1, counter)
        };
        aborts_if [inferred] aborts_of<make>(counter);
        aborts_if [inferred = sathard] exists x: u64: {
            let a = ..S1 |~ result_of<make>(counter);
            ..S1 |~ ensures_of<make>(counter, a, x)
        } && (S1 |~ aborts_of<make>(x));
        aborts_if [inferred = sathard] exists x: u64, y: u64: {
            let a = ..S1 |~ result_of<make>(counter);
            ..S1 |~ ensures_of<make>(counter, a, y)
        } && {
            let b = S1..S2 |~ result_of<make>(y);
            S1..S2 |~ ensures_of<make>(y, b, x)
        } && (S2 |~ aborts_of<make>(x));
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64: {
            let a = ..S1 |~ result_of<make>(counter);
            ..S1 |~ ensures_of<make>(counter, a, z)
        } && {
            let b = S1..S2 |~ result_of<make>(z);
            S1..S2 |~ ensures_of<make>(z, b, y)
        } && {
            let c = S2..S3 |~ result_of<make>(y);
            S2..S3 |~ ensures_of<make>(y, c, x)
        } && (S3 |~ aborts_of<make>(x));
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64: {
            let a = ..S1 |~ result_of<make>(counter);
            ..S1 |~ ensures_of<make>(counter, a, x1)
        } && {
            let b = S1..S2 |~ result_of<make>(x1);
            S1..S2 |~ ensures_of<make>(x1, b, z)
        } && {
            let c = S2..S3 |~ result_of<make>(z);
            S2..S3 |~ ensures_of<make>(z, c, y)
        } && {
            let d = S3..S4 |~ result_of<make>(y);
            S3..S4 |~ ensures_of<make>(y, d, x)
        } && (S4 |~ aborts_of<make>(x));
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64: {
            let a = ..S1 |~ result_of<make>(counter);
            ..S1 |~ ensures_of<make>(counter, a, x2)
        } && {
            let b = S1..S2 |~ result_of<make>(x2);
            S1..S2 |~ ensures_of<make>(x2, b, x1)
        } && {
            let c = S2..S3 |~ result_of<make>(x1);
            S2..S3 |~ ensures_of<make>(x1, c, z)
        } && {
            let d = S3..S4 |~ result_of<make>(z);
            S3..S4 |~ ensures_of<make>(z, d, y)
        } && {
            let e = S4..S5 |~ result_of<make>(y);
            S4..S5 |~ ensures_of<make>(y, e, x)
        } && (S5 |~ aborts_of<make>(x));
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64: {
            let a = ..S1 |~ result_of<make>(counter);
            ..S1 |~ ensures_of<make>(counter, a, x3)
        } && {
            let b = S1..S2 |~ result_of<make>(x3);
            S1..S2 |~ ensures_of<make>(x3, b, x2)
        } && {
            let c = S2..S3 |~ result_of<make>(x2);
            S2..S3 |~ ensures_of<make>(x2, c, x1)
        } && {
            let d = S3..S4 |~ result_of<make>(x1);
            S3..S4 |~ ensures_of<make>(x1, d, z)
        } && {
            let e = S4..S5 |~ result_of<make>(z);
            S4..S5 |~ ensures_of<make>(z, e, y)
        } && {
            let f = S5..S6 |~ result_of<make>(y);
            S5..S6 |~ ensures_of<make>(y, f, x)
        } && (S6 |~ aborts_of<make>(x));
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64, x4: u64: {
            let a = ..S1 |~ result_of<make>(counter);
            ..S1 |~ ensures_of<make>(counter, a, x4)
        } && {
            let b = S1..S2 |~ result_of<make>(x4);
            S1..S2 |~ ensures_of<make>(x4, b, x3)
        } && {
            let c = S2..S3 |~ result_of<make>(x3);
            S2..S3 |~ ensures_of<make>(x3, c, x2)
        } && {
            let d = S3..S4 |~ result_of<make>(x2);
            S3..S4 |~ ensures_of<make>(x2, d, x1)
        } && {
            let e = S4..S5 |~ result_of<make>(x1);
            S4..S5 |~ ensures_of<make>(x1, e, z)
        } && {
            let f = S5..S6 |~ result_of<make>(z);
            S5..S6 |~ ensures_of<make>(z, f, y)
        } && {
            let a_1 = S6..S7 |~ result_of<make>(y);
            S6..S7 |~ ensures_of<make>(y, a_1, x)
        } && (S7 |~ aborts_of<make>(x));
        aborts_if [inferred = sathard] exists x: u64, y: u64, z: u64, x1: u64, x2: u64, x3: u64, x4: u64, x5: u64: {
            let a = ..S1 |~ result_of<make>(counter);
            ..S1 |~ ensures_of<make>(counter, a, x5)
        } && {
            let b = S1..S2 |~ result_of<make>(x5);
            S1..S2 |~ ensures_of<make>(x5, b, x4)
        } && {
            let c = S2..S3 |~ result_of<make>(x4);
            S2..S3 |~ ensures_of<make>(x4, c, x3)
        } && {
            let d = S3..S4 |~ result_of<make>(x3);
            S3..S4 |~ ensures_of<make>(x3, d, x2)
        } && {
            let e = S4..S5 |~ result_of<make>(x2);
            S4..S5 |~ ensures_of<make>(x2, e, x1)
        } && {
            let f = S5..S6 |~ result_of<make>(x1);
            S5..S6 |~ ensures_of<make>(x1, f, z)
        } && {
            let a_1 = S6..S7 |~ result_of<make>(z);
            S6..S7 |~ ensures_of<make>(z, a_1, y)
        } && {
            let b_1 = S7..S8 |~ result_of<make>(y);
            S7..S8 |~ ensures_of<make>(y, b_1, x)
        } && (S8 |~ aborts_of<make>(x));
    }

}
/*
Verification: Succeeded.
*/
