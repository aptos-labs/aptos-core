spec aptos_std::math64 {

    spec max(a: u64, b: u64): u64 {
        pragma opaque = true;
        aborts_if false;
        ensures a >= b ==> result == a;
        ensures a < b ==> result == b;
    }

    spec min(a: u64, b: u64): u64 {
        pragma opaque = true;
        aborts_if false;
        ensures a < b ==> result == a;
        ensures a >= b ==> result == b;
    }

    spec average(a: u64, b: u64): u64 {
        pragma opaque;
        aborts_if false;
        ensures result == (a + b) / 2;
    }

    spec clamp(x: u64, lower: u64, upper: u64): u64 {
        pragma opaque = true;
        requires lower <= upper;
        aborts_if false;
        ensures lower <= x && x <= upper ==> result == x;
        ensures x < lower ==> result == lower;
        ensures upper < x ==> result == upper;
    }

    // The specs of `pow`, `floor_log2` and `sqrt` are validated with a smaller domain
    // in aptos-core/third_party/move/move-prover/tests/sources/functional/math8.move

    spec pow(n: u64, e: u64): u64 {
        pragma opaque;
        pragma verify = false;
        aborts_if spec_pow(n, e) > MAX_U64;
        ensures result == spec_pow(n, e);
    }

    spec floor_log2(x: u64): u8 {
        pragma opaque;
        pragma verify = false;
        aborts_if x == 0;
        ensures spec_pow(2, result) <= x;
        ensures x < spec_pow(2, result+1);
    }

    spec sqrt(x: u64): u64 {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures x > 0 ==> result * result <= x;
        ensures x > 0 ==> x < (result+1) * (result+1);
        ensures x == 0 ==> result == 0;
    }

    spec fun spec_pow(n: u64, e: u64): u64 {
        if (e == 0) {
            1
        }
        else {
            n * spec_pow(n, e-1)
        }
    }
}
