//# publish
module 0x42::m {

    public struct Scalar has copy, store, drop {
        data: vector<u8>
    }

}

//# publish
module 0x42::test_m {
    use std::vector;
    use 0x42::m::Scalar;

    /// Creates a Scalar from an u8.
    public fun new_scalar_from_u8(byte: u8): Scalar {
        let s = scalar_zero();
        let byte_zero = vector::borrow_mut(&mut s.data, 0);
        *byte_zero = byte;
        s
    }

    /// Returns 0 as a Scalar.
    public fun scalar_zero(): Scalar {
        Scalar {
            data: x"00"
        }
    }

}
