spec velor_std::math128 {

    spec max(a: u128, b: u128): u128 {
        aborts_if false;
        ensures a >= b ==> result == a;
        ensures a < b ==> result == b;
    }

    spec min(a: u128, b: u128): u128 {
        aborts_if false;
        ensures a < b ==> result == a;
        ensures a >= b ==> result == b;
    }

    spec average(a: u128, b: u128): u128 {
        pragma opaque;
        aborts_if false;
        ensures result == (a + b) / 2;
    }

    spec clamp(x: u128, lower: u128, upper: u128): u128 {
        requires (lower <= upper);
        aborts_if false;
        ensures (lower <=x && x <= upper) ==> result == x;
        ensures (x < lower) ==> result == lower;
        ensures (upper < x) ==> result == upper;
    }

    // The specs of `pow`, `floor_log2` and `sqrt` are validated with a smaller domain
    // in velor-core/third_party/move/move-prover/tests/sources/functional/math8.move

    spec pow(n: u128, e: u128): u128 {
        pragma opaque;
        aborts_if [abstract] spec_pow(n, e) > MAX_U128;
        ensures [abstract] result == spec_pow(n, e);
    }

    spec floor_log2(x: u128): u8 {
        pragma opaque;
        aborts_if [abstract] x == 0;
        ensures [abstract] spec_pow(2, result) <= x;
        ensures [abstract] x < spec_pow(2, result+1);
    }

    spec sqrt(x: u128): u128 {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] x > 0 ==> result * result <= x;
        ensures [abstract] x > 0 ==> x < (result+1) * (result+1);
    }

    spec fun spec_pow(n: u128, e: u128): u128 {
        if (e == 0) {
            1
        }
        else {
            n * spec_pow(n, e-1)
        }
    }
}
