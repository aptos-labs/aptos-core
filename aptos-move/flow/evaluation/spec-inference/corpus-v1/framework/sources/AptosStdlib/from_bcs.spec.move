spec aptos_std::from_bcs {
    // ----------------------------------
    // Uninterpreted functions and axioms
    // ----------------------------------
    spec module {
        // An uninterpreted function to represent the desrialization.
        fun deserialize<T>(bytes: vector<u8>): T;

        // Checks if `bytes` is valid so that it can be deserialized into type T.
        // This is modeled as an uninterpreted function.
        fun deserializable<T>(bytes: vector<u8>): bool;

        // `deserialize` is an injective function.
        // axiom<T> forall b1: vector<u8>, b2: vector<u8>:
        //    (deserialize<T>(b1) == deserialize<T>(b2) ==> b1 == b2);

        // If the input are equal, the result of deserialize should be equal too
        axiom<T> forall b1: vector<u8> , b2: vector<u8> :
            (b1 == b2 ==>
                deserializable<T>(b1) == deserializable<T>(b2));

        axiom<T> forall b1: vector<u8> , b2: vector<u8> :
            (b1 == b2 ==>
                deserialize<T>(b1) == deserialize<T>(b2));

        // `deserialize` is an inverse function of `bcs::serialize`.
        // TODO: disabled because this generic axiom causes a timeout.
        // axiom<T> forall v: T: deserialize<T>(bcs::serialize(v)) == v;

        // All serialized bytes are deserializable.
        // TODO: disabled because this generic axiom causes a timeout.
        // axiom<T> forall v: T: deserializable<T>(bcs::serialize(v));
    }

    // -----------------------
    // Function specifications
    // -----------------------

    spec from_bytes<T>(bytes: vector<u8>): T {
        pragma opaque;
        aborts_if !deserializable<T>(bytes);
        ensures result == deserialize<T>(bytes);
    }

    spec to_bytes(v: vector<u8>): vector<u8> {
        pragma opaque = true;
        ensures [inferred] result == deserialize<vector<u8>>(v);
        aborts_if [inferred] aborts_of<from_bytes<vector<u8>> >(v);
    }

    spec to_address(v: vector<u8>): address {
        pragma opaque = true;
        ensures [inferred] result == deserialize<address>(v);
        aborts_if [inferred] aborts_of<from_bytes<address>> (v);
    }

    spec to_bool(v: vector<u8>): bool {
        pragma opaque = true;
        ensures [inferred] result == deserialize<bool>(v);
        aborts_if [inferred] aborts_of<from_bytes<bool>> (v);
    }

    spec to_string(v: vector<u8>): 0x1::string::String {
        use 0x1::string;
        pragma opaque = true;
        ensures [inferred] string::spec_internal_check_utf8(
            string::bytes(deserialize<string::String>(v))
        ) ==>
            result == deserialize<string::String>(v);
        aborts_if [inferred]!string::spec_internal_check_utf8(
            string::bytes(deserialize<string::String>(v))
        );
        aborts_if [inferred] aborts_of<from_bytes<string::String>> (v);
    }

    spec to_u128(v: vector<u8>): u128 {
        pragma opaque = true;
        ensures [inferred] result == deserialize<u128>(v);
        aborts_if [inferred] aborts_of<from_bytes<u128>> (v);
    }

    spec to_u16(v: vector<u8>): u16 {
        pragma opaque = true;
        ensures [inferred] result == deserialize<u16>(v);
        aborts_if [inferred] aborts_of<from_bytes<u16>> (v);
    }

    spec to_u256(v: vector<u8>): u256 {
        pragma opaque = true;
        ensures [inferred] result == deserialize<u256>(v);
        aborts_if [inferred] aborts_of<from_bytes<u256>> (v);
    }

    spec to_u32(v: vector<u8>): u32 {
        pragma opaque = true;
        ensures [inferred] result == deserialize<u32>(v);
        aborts_if [inferred] aborts_of<from_bytes<u32>> (v);
    }

    spec to_u64(v: vector<u8>): u64 {
        pragma opaque = true;
        ensures [inferred] result == deserialize<u64>(v);
        aborts_if [inferred] aborts_of<from_bytes<u64>> (v);
    }

    spec to_u8(v: vector<u8>): u8 {
        pragma opaque = true;
        ensures [inferred] result == deserialize<u8>(v);
        aborts_if [inferred] aborts_of<from_bytes<u8>> (v);
    }
}
