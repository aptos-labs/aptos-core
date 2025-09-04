spec velor_std::velor_hash {
    spec module {
        /// `spec_sip_hash` is not assumed to be injective.
        fun spec_sip_hash(bytes: vector<u8>): u64;

        /// `spec_keccak256` is an injective function.
        fun spec_keccak256(bytes: vector<u8>): vector<u8>;
        axiom forall b1: vector<u8>, b2: vector<u8>:
            (spec_keccak256(b1) == spec_keccak256(b2) ==> b1 == b2);

        /// `spec_sha2_512_internal` is an injective function.
        fun spec_sha2_512_internal(bytes: vector<u8>): vector<u8>;
        axiom forall b1: vector<u8>, b2: vector<u8>:
            (spec_sha2_512_internal(b1) == spec_sha2_512_internal(b2) ==> b1 == b2);

        /// `spec_sha3_512_internal` is an injective function.
        fun spec_sha3_512_internal(bytes: vector<u8>): vector<u8>;
        axiom forall b1: vector<u8>, b2: vector<u8>:
            (spec_sha3_512_internal(b1) == spec_sha3_512_internal(b2) ==> b1 == b2);

        /// `spec_ripemd160_internal` is an injective function.
        fun spec_ripemd160_internal(bytes: vector<u8>): vector<u8>;
        axiom forall b1: vector<u8>, b2: vector<u8>:
            (spec_ripemd160_internal(b1) == spec_ripemd160_internal(b2) ==> b1 == b2);

        /// `spec_blake2b_256_internal` is an injective function.
        fun spec_blake2b_256_internal(bytes: vector<u8>): vector<u8>;
        axiom forall b1: vector<u8>, b2: vector<u8>:
            (spec_blake2b_256_internal(b1) == spec_blake2b_256_internal(b2) ==> b1 == b2);
    }

    spec sip_hash(bytes: vector<u8>): u64 {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_sip_hash(bytes);
    }

    spec sip_hash_from_value<MoveValue>(v: &MoveValue): u64 {
        pragma opaque;
        ensures result == spec_sip_hash(bcs::serialize(v));
    }

    spec keccak256(bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_keccak256(bytes);
    }

    spec sha2_512_internal(bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_sha2_512_internal(bytes);
    }

    spec sha3_512_internal(bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_sha3_512_internal(bytes);
    }

    spec ripemd160_internal(bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_ripemd160_internal(bytes);
    }

    spec sha2_512(bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if !features::spec_is_enabled(features::SHA_512_AND_RIPEMD_160_NATIVES);
        ensures result == spec_sha2_512_internal(bytes);
    }

    spec sha3_512(bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if !features::spec_is_enabled(features::SHA_512_AND_RIPEMD_160_NATIVES);
        ensures result == spec_sha3_512_internal(bytes);
    }

    spec ripemd160(bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if !features::spec_is_enabled(features::SHA_512_AND_RIPEMD_160_NATIVES);
        ensures result == spec_ripemd160_internal(bytes);
    }

    spec blake2b_256_internal(bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if false;
        ensures result == spec_blake2b_256_internal(bytes);
    }

    spec blake2b_256(bytes: vector<u8>): vector<u8> {
        pragma opaque;
        aborts_if !features::spec_is_enabled(features::BLAKE2B_256_NATIVE);
        ensures result == spec_blake2b_256_internal(bytes);
    }

}
