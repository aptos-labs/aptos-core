// Port of the Are We Fast Yet `Sieve` benchmark.
//
// This code is based on the SOM class library.
//
// Copyright (c) 2001-2016 see AUTHORS.md file
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the 'Software'), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED 'AS IS', WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

module bench::sieve {
    use std::vector;

    /// The kernel result did not match the `expected` argument.
    const EBAD_RESULT: u64 = 1;

    /// The Java gets this from `new boolean[size]` plus `Arrays.fill(_, true)`.
    fun new_flags(size: u64): vector<bool> {
        let flags = vector::empty<bool>();
        let i = 0;
        while (i < size) {
            vector::push_back(&mut flags, true);
            i = i + 1;
        };
        flags
    }

    /// Count the primes in `[2, size]`, clearing the flag of every composite.
    /// Flag `i - 1` holds the primality of `i`, so index 0 is never inspected.
    /// AWFY indexes this way and the port keeps it.
    fun sieve(flags: &mut vector<bool>, size: u64): u64 {
        let prime_count = 0;
        let i = 2;
        while (i <= size) {
            if (*vector::borrow(flags, i - 1)) {
                prime_count = prime_count + 1;
                let k = i + i;
                while (k <= size) {
                    *vector::borrow_mut(flags, k - 1) = false;
                    k = k + i;
                };
            };
            i = i + 1;
        };
        prime_count
    }

    /// Sieve `[2, size]` from scratch `iters` times and sum the prime counts.
    /// AWFY reallocates the flags per round, so each round does equal work.
    public fun bench_sieve(size: u64, iters: u64): u64 {
        let acc = 0;
        let round = 0;
        while (round < iters) {
            let flags = new_flags(size);
            acc = acc + sieve(&mut flags, size);
            round = round + 1;
        };
        acc
    }

    public entry fun run(_s: &signer, size: u64, iters: u64, expected: u64) {
        assert!(bench_sieve(size, iters) == expected, EBAD_RESULT);
    }

    // The AWFY setting and its reference result.
    #[test]
    fun test_awfy_reference() {
        assert!(bench_sieve(5000, 1) == 669, 0);
    }

    #[test]
    fun test_size_100() {
        assert!(bench_sieve(100, 1) == 25, 0);
    }

    #[test]
    fun test_size_1000() {
        assert!(bench_sieve(1000, 1) == 168, 0);
    }

    #[test]
    fun test_size_10000() {
        assert!(bench_sieve(10000, 1) == 1229, 0);
    }

    #[test]
    fun test_iters_scale() {
        assert!(bench_sieve(5000, 3) == 3 * 669, 0);
    }

    // Below 2 there is nothing to inspect, so the loop never runs.
    #[test]
    fun test_degenerate_sizes() {
        assert!(bench_sieve(0, 1) == 0, 0);
        assert!(bench_sieve(1, 1) == 0, 0);
        assert!(bench_sieve(2, 1) == 1, 0);
        assert!(bench_sieve(5000, 0) == 0, 0);
    }

    #[test(s = @bench)]
    fun test_run(s: &signer) {
        run(s, 5000, 1, 669);
    }

    #[test(s = @bench)]
    #[expected_failure(abort_code = EBAD_RESULT, location = Self)]
    fun test_run_bad_expected(s: &signer) {
        run(s, 5000, 1, 668);
    }
}
