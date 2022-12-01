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
        // TODO: disabled due to the issue with generic axioms.
        // axiom<T> forall b1: vector<u8>, b2: vector<u8>:
        //     (deserialize<T>(b1) == deserialize<T>(b2) ==> b1 == b2);

        // `deserialize` is an inverse function of `bcs::serialize`.
        // TODO: disabled due to the issue with generic axioms.
        // axiom<T> forall v: T: deserialize<T>(bcs::serialize(v)) == v;

        // All serialized bytes are deserializable.
        // TODO: disabled due to the issue with generic axioms.
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
