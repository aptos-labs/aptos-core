module 0x42::comparator {
    use std::vector;

    const EQUAL: u8 = 0;
    const SMALLER: u8 = 1;
    const GREATER: u8 = 2;

    struct Result has drop {
        inner: u8,
    }

    spec compare_u8_vector(left: vector<u8>, right: vector<u8>): Result {
        pragma unroll = 5;

        let left_length = len(left);
        let right_length = len(right);

        ensures (result.inner == EQUAL) ==> (
            (left_length == right_length) &&
                (forall i: u64 where i < left_length: left[i] == right[i])
        );
    }

    // Performs a comparison of two vector<u8>s or byte vectors
    public fun compare_u8_vector(left: vector<u8>, right: vector<u8>): Result {
        let left_length = vector::length(&left);
        let right_length = vector::length(&right);

        let idx = 0;

        while (idx < left_length && idx < right_length) {
            let left_byte = *vector::borrow(&left, idx);
            let right_byte = *vector::borrow(&right, idx);

            if (left_byte < right_byte) {
                return Result { inner: SMALLER }
            } else if (left_byte > right_byte) {
                return Result { inner: GREATER }
            };
            idx = idx + 1;
        };

        if (left_length < right_length) {
            Result { inner: SMALLER }
        } else if (left_length > right_length) {
            Result { inner: GREATER }
        } else {
            Result { inner: EQUAL }
        }
    }

}
