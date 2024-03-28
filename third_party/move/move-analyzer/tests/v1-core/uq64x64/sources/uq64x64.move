/// Implementation of FixedPoint u64 in Move language.
module uq64x64::uq64x64 {
    // Error codes.

    /// When divide by zero attempted.
    const ERR_DIVIDE_BY_ZERO: u64 = 100;

    // Constants.

    const Q64: u128 = 18446744073709551615;

    /// When a and b are equals.
    const EQUAL: u8 = 0;

    /// When a is less than b equals.
    const LESS_THAN: u8 = 1;

    /// When a is greater than b.
    const GREATER_THAN: u8 = 2;

    /// The resource to store `UQ64x64`.
    struct UQ64x64 has copy, store, drop {
        v: u128
    }

    /// Encode `u64` to `UQ64x64`
    public fun encode(x: u64): UQ64x64 {
        let v = (x as u128) * Q64;
        UQ64x64{ v }
    }
    spec encode {
        ensures Q64 == MAX_U64;
        ensures result.v == x * Q64;
        ensures result.v <= MAX_U128;
    }

    /// Decode a `UQ64x64` into a `u64` by truncating after the radix point.
    public fun decode(uq: UQ64x64): u64 {
        ((uq.v / Q64) as u64)
    }
    spec decode {
        ensures result == uq.v / Q64;
    }

    /// Get `u128` from UQ64x64
    public fun to_u128(uq: UQ64x64): u128 {
        uq.v
    }
    spec to_u128 {
        ensures result == uq.v;
    }

    /// Multiply a `UQ64x64` by a `u64`, returning a `UQ64x64`
    public fun mul(uq: UQ64x64, y: u64): UQ64x64 {
        // vm would direct abort when overflow occured
        let v = uq.v * (y as u128);

        UQ64x64{ v }
    }
    spec mul {
        ensures result.v == uq.v * y;
    }

    /// Divide a `UQ64x64` by a `u128`, returning a `UQ64x64`.
    public fun div(uq: UQ64x64, y: u64): UQ64x64 {
        assert!(y != 0, ERR_DIVIDE_BY_ZERO);

        let v = uq.v / (y as u128);
        UQ64x64{ v }
    }
    spec div {
        aborts_if y == 0 with ERR_DIVIDE_BY_ZERO;
        ensures result.v == uq.v / y;
    }

    /// Returns a `UQ64x64` which represents the ratio of the numerator to the denominator.
    public fun fraction(numerator: u64, denominator: u64): UQ64x64 {
        assert!(denominator != 0, ERR_DIVIDE_BY_ZERO);

        let r = (numerator as u128) * Q64;
        let v = r / (denominator as u128);

        UQ64x64{ v }
    }
    spec fraction {
        aborts_if denominator == 0 with ERR_DIVIDE_BY_ZERO;
        ensures result.v == numerator * Q64 / denominator;
    }

    /// Compare two `UQ64x64` numbers.
    public fun compare(left: &UQ64x64, right: &UQ64x64): u8 {
        if (left.v == right.v) {
            return EQUAL
        } else if (left.v < right.v) {
            return LESS_THAN
        } else {
            return GREATER_THAN
        }
    }
    spec compare {
        ensures left.v == right.v ==> result == EQUAL;
        ensures left.v < right.v ==> result == LESS_THAN;
        ensures left.v > right.v ==> result == GREATER_THAN;
    }

    /// Check if `UQ64x64` is zero
    public fun is_zero(uq: &UQ64x64): bool {
        uq.v == 0
    }
    spec is_zero {
        ensures uq.v == 0 ==> result == true;
        ensures uq.v > 0 ==> result == false;
    }
}