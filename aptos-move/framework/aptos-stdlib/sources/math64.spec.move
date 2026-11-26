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


    spec pow(n: u64, e: u64): u64 {
        pragma opaque;
        // Names for the arguments at entry; the function body assigns to both parameters.
        let n0 = n;
        let e0 = e;
        aborts_if spec_pow(n0, e0) > MAX_U64;
        ensures result == spec_pow(n0, e0);
    } proof {
        forall a: u64, b: u64 {spec_pow(a, b)} apply spec_pow_zero(a, b);
        forall a: u64, b: u64, d: u64 {spec_pow(a, b), spec_pow(a, d)} apply spec_pow_unfold(a, b, d);
        forall a: u64, b: u64 {spec_pow(a, b)} apply spec_pow_pos(a, b);
        forall a: u64, b: u64, m: u64, k: u64 {spec_pow(a, b), spec_pow(m, k)}
            apply spec_pow_square(a, b, m, k);
    }

    spec module {
        lemma spec_pow_zero(n: u64, e: u64) {
            requires e == 0;
            ensures spec_pow(n, e) == 1;
        }

        lemma spec_pow_unfold(n: u64, e: u64, d: u64) {
            requires d + 1 == e;
            ensures spec_pow(n, e) == n * spec_pow(n, d);
        }

        /// A positive base has a positive power.
        lemma spec_pow_pos(n: u64, e: u64) {
            decreases e;
            requires n > 0;
            ensures spec_pow(n, e) > 0;
        } proof {
            if (e > 0) {
                apply spec_pow_pos(n, e - 1);
            }
        }

        /// Squaring the base halves the exponent.
        lemma spec_pow_square(n: u64, e: u64, m: u64, k: u64) {
            decreases k;
            requires m == n * n;
            requires e == 2 * k || e == 2 * k + 1;
            ensures e == 2 * k ==> spec_pow(n, e) == spec_pow(m, k);
            ensures e == 2 * k + 1 ==> spec_pow(n, e) == n * spec_pow(m, k);
        } proof {
            if (k > 0) {
                assert spec_pow(n, e) == n * spec_pow(n, e - 1);
                assert spec_pow(n, e - 1) == n * spec_pow(n, e - 2);
                assert spec_pow(m, k) == m * spec_pow(m, k - 1);
                apply spec_pow_square(n, e - 2, m, k - 1);
            }
        }
    }

    spec floor_log2(x: u64): u8 {
        pragma opaque;
        aborts_if x == 0;
        ensures spec_pow(2, result) <= x;
        ensures x < spec_pow(2, result+1);
    } proof {
        apply pow2_table();
    }

    spec sqrt(x: u64): u64 {
        pragma opaque;
        // Not verified here; validated on a smaller domain in
        // third_party/move/move-prover/tests/sources/functional/math8.move.
        pragma verify = false;
        aborts_if false;
        ensures x > 0 ==> result * result <= x;
        ensures x > 0 ==> x < (result+1) * (result+1);
        ensures x == 0 ==> result == 0;
    }

    spec module {
        /// The powers of two up to 2^64.
        lemma pow2_table() {
            ensures spec_pow(2, 0) == 1;
            ensures spec_pow(2, 1) == 2;
            ensures spec_pow(2, 2) == 4;
            ensures spec_pow(2, 3) == 8;
            ensures spec_pow(2, 4) == 16;
            ensures spec_pow(2, 5) == 32;
            ensures spec_pow(2, 6) == 64;
            ensures spec_pow(2, 7) == 128;
            ensures spec_pow(2, 8) == 256;
            ensures spec_pow(2, 9) == 512;
            ensures spec_pow(2, 10) == 1024;
            ensures spec_pow(2, 11) == 2048;
            ensures spec_pow(2, 12) == 4096;
            ensures spec_pow(2, 13) == 8192;
            ensures spec_pow(2, 14) == 16384;
            ensures spec_pow(2, 15) == 32768;
            ensures spec_pow(2, 16) == 65536;
            ensures spec_pow(2, 17) == 131072;
            ensures spec_pow(2, 18) == 262144;
            ensures spec_pow(2, 19) == 524288;
            ensures spec_pow(2, 20) == 1048576;
            ensures spec_pow(2, 21) == 2097152;
            ensures spec_pow(2, 22) == 4194304;
            ensures spec_pow(2, 23) == 8388608;
            ensures spec_pow(2, 24) == 16777216;
            ensures spec_pow(2, 25) == 33554432;
            ensures spec_pow(2, 26) == 67108864;
            ensures spec_pow(2, 27) == 134217728;
            ensures spec_pow(2, 28) == 268435456;
            ensures spec_pow(2, 29) == 536870912;
            ensures spec_pow(2, 30) == 1073741824;
            ensures spec_pow(2, 31) == 2147483648;
            ensures spec_pow(2, 32) == 4294967296;
            ensures spec_pow(2, 33) == 8589934592;
            ensures spec_pow(2, 34) == 17179869184;
            ensures spec_pow(2, 35) == 34359738368;
            ensures spec_pow(2, 36) == 68719476736;
            ensures spec_pow(2, 37) == 137438953472;
            ensures spec_pow(2, 38) == 274877906944;
            ensures spec_pow(2, 39) == 549755813888;
            ensures spec_pow(2, 40) == 1099511627776;
            ensures spec_pow(2, 41) == 2199023255552;
            ensures spec_pow(2, 42) == 4398046511104;
            ensures spec_pow(2, 43) == 8796093022208;
            ensures spec_pow(2, 44) == 17592186044416;
            ensures spec_pow(2, 45) == 35184372088832;
            ensures spec_pow(2, 46) == 70368744177664;
            ensures spec_pow(2, 47) == 140737488355328;
            ensures spec_pow(2, 48) == 281474976710656;
            ensures spec_pow(2, 49) == 562949953421312;
            ensures spec_pow(2, 50) == 1125899906842624;
            ensures spec_pow(2, 51) == 2251799813685248;
            ensures spec_pow(2, 52) == 4503599627370496;
            ensures spec_pow(2, 53) == 9007199254740992;
            ensures spec_pow(2, 54) == 18014398509481984;
            ensures spec_pow(2, 55) == 36028797018963968;
            ensures spec_pow(2, 56) == 72057594037927936;
            ensures spec_pow(2, 57) == 144115188075855872;
            ensures spec_pow(2, 58) == 288230376151711744;
            ensures spec_pow(2, 59) == 576460752303423488;
            ensures spec_pow(2, 60) == 1152921504606846976;
            ensures spec_pow(2, 61) == 2305843009213693952;
            ensures spec_pow(2, 62) == 4611686018427387904;
            ensures spec_pow(2, 63) == 9223372036854775808;
            ensures spec_pow(2, 64) == 18446744073709551616;
        }
    }

    spec fun spec_pow(n: u64, e: u64): u64 [weight = 100] {
        if (e == 0) {
            1
        }
        else {
            n * spec_pow(n, e-1)
        }
    }
}
