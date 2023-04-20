spec aptos_std::math64 {

    spec max(a: u64, b: u64): u64 {
        aborts_if false;
        ensures a >= b ==> result == a;
        ensures a < b ==> result == b;
    }

    spec min(a: u64, b: u64): u64 {
        aborts_if false;
        ensures a < b ==> result == a;
        ensures a >= b ==> result == b;
    }

    spec average(a: u64, b: u64): u64 {
        pragma opaque;
        aborts_if false;
        ensures result == (a + b) / 2;
    }

    spec pow(n: u64, e: u64): u64 {
        pragma opaque;
        // TODO: verify the spec.
        aborts_if [abstract] spec_pow(n, e) > MAX_U64;
        ensures [abstract] result == spec_pow(n, e);
    }

    spec fun spec_pow(e: u64, n: u64): u64 {
        if (e == 0) {
            1
        }
        else {
            n * spec_pow(n, e-1)
        }
    }
}
