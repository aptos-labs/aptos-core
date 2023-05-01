spec aptos_std::math128 {

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

    spec pow(n: u128, e: u128): u128 {
        pragma opaque;
        // TODO: verify the spec.
        aborts_if [abstract] spec_pow(n, e) > MAX_U128;
        ensures [abstract] result == spec_pow(n, e);
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
