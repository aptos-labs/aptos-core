// Port of the Are We Fast Yet `Bounce` benchmark and the `som.Random`
// generator that feeds it. Both are MIT licensed; the notice follows.
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

module bench::bounce {
    use std::vector;

    /// `run` was given an `expected` the kernel did not produce.
    const EWRONG_RESULT: u64 = 1;

    // `som.Random`: a 16-bit LCG. The reference bounce count only reproduces
    // with this exact stream.
    const RANDOM_SEED: u64 = 74755;
    const RANDOM_MUL: u64 = 1309;
    const RANDOM_INC: u64 = 13849;
    const RANDOM_MASK: u64 = 65535;

    // Spawn box and velocity range, from AWFY's `Bounce`.
    const POS_RANGE: u64 = 500;
    const VEL_RANGE: u64 = 300;
    const VEL_BIAS: i64 = 150;
    const LIMIT: i64 = 500;

    struct Ball has copy, drop, store {
        x: i64,
        y: i64,
        x_vel: i64,
        y_vel: i64,
    }

    /// One draw from `som.Random`. The mask keeps the result in [0, 65535], so
    /// the caller's `%` never sees a negative dividend and can stay in `u64`.
    fun random_next(seed: &mut u64): u64 {
        *seed = ((*seed * RANDOM_MUL) + RANDOM_INC) & RANDOM_MASK;
        *seed
    }

    /// Four consecutive draws, in AWFY's order: x, y, x_vel, y_vel.
    fun new_ball(seed: &mut u64): Ball {
        let x = ((random_next(seed) % POS_RANGE) as i64);
        let y = ((random_next(seed) % POS_RANGE) as i64);
        let x_vel = ((random_next(seed) % VEL_RANGE) as i64) - VEL_BIAS;
        let y_vel = ((random_next(seed) % VEL_RANGE) as i64) - VEL_BIAS;
        Ball { x, y, x_vel, y_vel }
    }

    /// Java's `Math.abs`. Velocities start in [-150, 149] and the wall clamps
    /// only ever flip their sign, so `0 - v` cannot overflow here.
    fun abs(v: i64): i64 {
        if (v < 0) { 0 - v } else { v }
    }

    /// Advance one ball and clamp it back into the box, returning whether it
    /// hit a wall. A step that hits an x wall and a y wall still counts as one
    /// bounce; tallying each clamp instead gives 1426 rather than 1331.
    fun bounce(ball: &mut Ball): bool {
        let bounced = false;
        ball.x = ball.x + ball.x_vel;
        ball.y = ball.y + ball.y_vel;
        if (ball.x > LIMIT) {
            ball.x = LIMIT;
            ball.x_vel = 0 - abs(ball.x_vel);
            bounced = true;
        };
        if (ball.x < 0) {
            ball.x = 0;
            ball.x_vel = abs(ball.x_vel);
            bounced = true;
        };
        if (ball.y > LIMIT) {
            ball.y = LIMIT;
            ball.y_vel = 0 - abs(ball.y_vel);
            bounced = true;
        };
        if (ball.y < 0) {
            ball.y = 0;
            ball.y_vel = abs(ball.y_vel);
            bounced = true;
        };
        bounced
    }

    /// One AWFY round: build the balls from a single fresh generator, then step
    /// every ball `steps` times.
    fun run_round(ball_count: u64, steps: u64): u64 {
        let seed = RANDOM_SEED;
        let balls = vector::empty<Ball>();
        let i = 0;
        while (i < ball_count) {
            vector::push_back(&mut balls, new_ball(&mut seed));
            i = i + 1;
        };

        let bounces = 0;
        let step = 0;
        while (step < steps) {
            let b = 0;
            while (b < ball_count) {
                if (bounce(vector::borrow_mut(&mut balls, b))) {
                    bounces = bounces + 1;
                };
                b = b + 1;
            };
            step = step + 1;
        };
        bounces
    }

    /// Total bounces over `iters` rounds. Every round restarts the generator,
    /// so the result is `iters` times the single-round count.
    public fun bench_bounce(ball_count: u64, steps: u64, iters: u64): u64 {
        let total = 0;
        let i = 0;
        while (i < iters) {
            total = total + run_round(ball_count, steps);
            i = i + 1;
        };
        total
    }

    public entry fun run(
        _s: &signer,
        ball_count: u64,
        steps: u64,
        iters: u64,
        expected: u64,
    ) {
        assert!(bench_bounce(ball_count, steps, iters) == expected, EWRONG_RESULT);
    }

    #[test_only]
    /// The first `n` draws of `som.Random`, for the conformance test below.
    public fun random_draws(n: u64): vector<u64> {
        let seed = RANDOM_SEED;
        let draws = vector::empty<u64>();
        let i = 0;
        while (i < n) {
            vector::push_back(&mut draws, random_next(&mut seed));
            i = i + 1;
        };
        draws
    }

    #[test]
    fun test_random_first_nine() {
        let expected =
            vector[22896u64, 34761, 34014, 39231, 52540, 41445, 1546, 5947, 65224];
        assert!(random_draws(9) == expected, 0);
    }

    #[test]
    fun test_awfy_reference() {
        assert!(bench_bounce(100, 50, 1) == 1331, 0);
    }

    #[test]
    fun test_two_rounds() {
        assert!(bench_bounce(100, 50, 2) == 2662, 0);
    }

    #[test]
    fun test_one_ball() {
        assert!(bench_bounce(1, 10, 1) == 2, 0);
    }

    #[test]
    fun test_few_balls() {
        assert!(bench_bounce(7, 13, 1) == 21, 0);
    }

    #[test]
    fun test_empty_knobs() {
        assert!(bench_bounce(0, 50, 1) == 0, 0);
        assert!(bench_bounce(100, 0, 1) == 0, 1);
        assert!(bench_bounce(100, 50, 0) == 0, 2);
    }

    #[test(s = @bench)]
    fun test_run(s: &signer) {
        run(s, 100, 50, 1, 1331);
    }

    #[test(s = @bench)]
    #[expected_failure(abort_code = EWRONG_RESULT, location = Self)]
    fun test_run_wrong_expected(s: &signer) {
        run(s, 100, 50, 1, 1330);
    }
}
