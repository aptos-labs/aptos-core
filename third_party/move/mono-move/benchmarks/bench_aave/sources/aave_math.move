// Ported from https://github.com/aave/aptos-aave-v3, modules
// `aave_math::wad_ray_math` and `aave_math::math_utils`.
// Copyright (c) Aave DAO and contributors.
// SPDX-License-Identifier: Apache-2.0

/// Fixed-point arithmetic. WAD carries 18 decimals, RAY carries 27, and
/// percentages are basis points out of 10000. All rounding is half-up unless
/// the name says otherwise.
module bench::aave_math {
    use aptos_framework::timestamp;

    const U256_MAX: u256 =
        0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;

    const WAD: u256 = 1_000_000_000_000_000_000;
    const HALF_WAD: u256 = 500_000_000_000_000_000;
    const RAY: u256 = 1_000_000_000_000_000_000_000_000_000;
    const HALF_RAY: u256 = 500_000_000_000_000_000_000_000_000;
    const WAD_RAY_RATIO: u256 = 1_000_000_000;

    const PERCENTAGE_FACTOR: u256 = 10_000;
    const HALF_PERCENTAGE_FACTOR: u256 = 5_000;

    /// Leap years are ignored, matching the upstream rate model.
    const SECONDS_PER_YEAR: u256 = 31_536_000;

    const EOVERFLOW: u64 = 1;
    const EDIVISION_BY_ZERO: u64 = 2;

    public fun u256_max(): u256 {
        U256_MAX
    }

    public fun wad(): u256 {
        WAD
    }

    public fun ray(): u256 {
        RAY
    }

    public fun percentage_factor(): u256 {
        PERCENTAGE_FACTOR
    }

    public fun seconds_per_year(): u256 {
        SECONDS_PER_YEAR
    }

    public fun wad_mul(a: u256, b: u256): u256 {
        if (b == 0) {
            return 0
        };
        assert!(a <= (U256_MAX - HALF_WAD) / b, EOVERFLOW);
        (a * b + HALF_WAD) / WAD
    }

    public fun wad_div(a: u256, b: u256): u256 {
        assert!(b > 0, EDIVISION_BY_ZERO);
        if (a == 0) {
            return 0
        };
        assert!(a <= (U256_MAX - b / 2) / WAD, EOVERFLOW);
        (a * WAD + b / 2) / b
    }

    public fun ray_mul(a: u256, b: u256): u256 {
        if (a == 0 || b == 0) {
            return 0
        };
        assert!(a <= (U256_MAX - HALF_RAY) / b, EOVERFLOW);
        (a * b + HALF_RAY) / RAY
    }

    public fun ray_mul_up(a: u256, b: u256): u256 {
        if (a == 0 || b == 0) {
            return 0
        };
        assert!(a <= (U256_MAX - RAY + 1) / b, EOVERFLOW);
        (a * b + RAY - 1) / RAY
    }

    public fun ray_mul_down(a: u256, b: u256): u256 {
        if (a == 0 || b == 0) {
            return 0
        };
        (a * b) / RAY
    }

    public fun ray_div(a: u256, b: u256): u256 {
        assert!(b > 0, EDIVISION_BY_ZERO);
        if (a == 0) {
            return 0
        };
        assert!(a <= (U256_MAX - b / 2) / RAY, EOVERFLOW);
        (a * RAY + b / 2) / b
    }

    public fun ray_div_up(a: u256, b: u256): u256 {
        assert!(b > 0, EDIVISION_BY_ZERO);
        if (a == 0) {
            return 0
        };
        assert!(a <= (U256_MAX - b + 1) / RAY, EOVERFLOW);
        (a * RAY + b - 1) / b
    }

    public fun ray_div_down(a: u256, b: u256): u256 {
        assert!(b > 0, EDIVISION_BY_ZERO);
        if (a == 0) {
            return 0
        };
        (a * RAY) / b
    }

    public fun ray_to_wad(a: u256): u256 {
        let b = a / WAD_RAY_RATIO;
        if (a % WAD_RAY_RATIO >= WAD_RAY_RATIO / 2) {
            b = b + 1;
        };
        b
    }

    public fun wad_to_ray(a: u256): u256 {
        assert!(a <= U256_MAX / WAD_RAY_RATIO, EOVERFLOW);
        a * WAD_RAY_RATIO
    }

    public fun percent_mul(value: u256, percentage: u256): u256 {
        if (value == 0 || percentage == 0) {
            return 0
        };
        assert!(
            value <= (U256_MAX - HALF_PERCENTAGE_FACTOR) / percentage, EOVERFLOW
        );
        (value * percentage + HALF_PERCENTAGE_FACTOR) / PERCENTAGE_FACTOR
    }

    public fun percent_div(value: u256, percentage: u256): u256 {
        assert!(percentage > 0, EDIVISION_BY_ZERO);
        assert!(value <= (U256_MAX - percentage / 2) / PERCENTAGE_FACTOR, EOVERFLOW);
        (value * PERCENTAGE_FACTOR + percentage / 2) / percentage
    }

    public fun pow(base: u256, exponent: u256): u256 {
        let result = 1;
        while (exponent > 0) {
            if (exponent & 1 == 1) {
                result = result * base;
            };
            base = base * base;
            exponent = exponent >> 1;
        };
        result
    }

    public fun ceil_div(numerator: u256, denominator: u256): u256 {
        assert!(denominator > 0, EDIVISION_BY_ZERO);
        if (numerator == 0) {
            return 0
        };
        (numerator + denominator - 1) / denominator
    }

    public fun min(a: u256, b: u256): u256 {
        if (a <= b) { a } else { b }
    }

    /// Simple interest accrued on the supply side since `last_update_timestamp`.
    public fun calculate_linear_interest(
        rate: u256, last_update_timestamp: u64
    ): u256 {
        let time_passed = timestamp::now_seconds() - last_update_timestamp;
        RAY + (rate * (time_passed as u256)) / SECONDS_PER_YEAR
    }

    /// Third-order Taylor expansion of e^(r*t). Exact exponentiation is too
    /// expensive on chain, and the truncation slightly favours the protocol.
    ///
    /// Solidity Aave V3 expands (1+rate)^t binomially instead, so the two
    /// protocols disagree here. This follows the Aptos port.
    public fun calculate_compounded_interest(
        rate: u256, last_update_timestamp: u64, current_timestamp: u64
    ): u256 {
        assert!(current_timestamp >= last_update_timestamp, EOVERFLOW);
        let s = ((current_timestamp - last_update_timestamp) as u256);
        if (s == 0) {
            return RAY
        };
        let x = s * rate / SECONDS_PER_YEAR;
        RAY + x + ray_mul(x, (x / 2 + ray_mul(x, x / 6)))
    }

    public fun calculate_compounded_interest_now(
        rate: u256, last_update_timestamp: u64
    ): u256 {
        calculate_compounded_interest(
            rate, last_update_timestamp, timestamp::now_seconds()
        )
    }

    /// Flips every bit; Move has no unary bitwise negation operator.
    public fun bitwise_negation(x: u256): u256 {
        x ^ U256_MAX
    }
}
