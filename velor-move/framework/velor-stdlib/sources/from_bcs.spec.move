spec velor_std::from_bcs {
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
        axiom<T> forall b1: vector<u8>, b2: vector<u8>:
            ( b1 == b2 ==> deserializable<T>(b1) == deserializable<T>(b2) );

        axiom<T> forall b1: vector<u8>, b2: vector<u8>:
            ( b1 == b2 ==> deserialize<T>(b1) == deserialize<T>(b2) );

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
}
