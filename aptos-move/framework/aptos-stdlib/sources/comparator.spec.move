spec aptos_std::comparator {
    spec Result {
        invariant inner == EQUAL || inner == SMALLER || inner == GREATER;
    }

    spec is_equal(self: &Result): bool {
        aborts_if false;
        let res = self;
        ensures result == (res.inner == EQUAL);
    }

    spec is_smaller_than(self: &Result): bool {
        aborts_if false;
        let res = self;
        ensures result == (res.inner == SMALLER);
    }

    spec is_greater_than(self: &Result): bool {
        aborts_if false;
        let res = self;
        ensures result == (res.inner == GREATER);
    }

    spec compare<T>(left: &T, right: &T): Result {
        let left_bytes = bcs::to_bytes(left);
        let right_bytes = bcs::to_bytes(right);
        ensures result == spec_compare_u8_vector(left_bytes, right_bytes);
    }

    spec fun spec_compare_u8_vector(left: vector<u8>, right: vector<u8>): Result;

    spec compare_u8_vector(left: vector<u8>, right: vector<u8>): Result {
        pragma unroll = 5;
        pragma opaque;
        aborts_if false;

        let left_length = len(left);
        let right_length = len(right);

        ensures (result.inner == EQUAL) ==> (
            (left_length == right_length) &&
                (forall i: u64 where i < left_length: left[i] == right[i])
        );

        ensures (result.inner == SMALLER) ==> (
            (exists i: u64 where i < left_length:
                (i < right_length) &&
                    (left[i] < right[i]) &&
                    (forall j: u64 where j < i: left[j] == right[j])
            ) ||
                (left_length < right_length)
        );

        ensures (result.inner == GREATER) ==> (
            (exists i: u64 where i < left_length:
                (i < right_length) &&
                    (left[i] > right[i]) &&
                    (forall j: u64 where j < i: left[j] == right[j])
            ) ||
                (left_length > right_length)
        );

        ensures [abstract] result == spec_compare_u8_vector(left, right);
    }
}
