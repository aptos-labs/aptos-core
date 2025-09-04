module poc::aggregate_pubkeys_internal {
    use velor_std::bls12381::{Self, PublicKeyWithPoP, AggrPublicKeysWithPoP};
    use std::vector;
    use std::option::{Self};

    public entry fun main(_owner: &signer) {
        let pk_bytes1 = x"808864c91ae7a9998b3f5ee71f447840864e56d79838e4785ff5126c51480198df3d972e1e0348c6da80d396983e42d7";
        let pop_bytes1 = x"ab42afff92510034bf1232a37a0d31bc8abfc17e7ead9170d2d100f6cf6c75ccdcfedbd31699a112b4464a06fd636f3f190595863677d660b4c5d922268ace421f9e86e3a054946ee34ce29e1f88c1a10f27587cf5ec528d65ba7c0dc4863364";

        let pop1 = bls12381::proof_of_possession_from_bytes(pop_bytes1);
        let pk_with_pop1_opt = bls12381::public_key_from_bytes_with_pop(pk_bytes1, &pop1);
        assert!(option::is_some(&pk_with_pop1_opt), 1);
        let pk_with_pop1 = option::extract(&mut pk_with_pop1_opt);

        let pk_bytes2 = x"808864c91ae7a9998b3f5ee71f447840864e56d79838e4785ff5126c51480198df3d972e1e0348c6da80d396983e42d7";
        let pop_bytes2 = x"ab42afff92510034bf1232a37a0d31bc8abfc17e7ead9170d2d100f6cf6c75ccdcfedbd31699a112b4464a06fd636f3f190595863677d660b4c5d922268ace421f9e86e3a054946ee34ce29e1f88c1a10f27587cf5ec528d65ba7c0dc4863364";

        let pop2 = bls12381::proof_of_possession_from_bytes(pop_bytes2);
        let pk_with_pop2_opt = bls12381::public_key_from_bytes_with_pop(pk_bytes2, &pop2);
        assert!(option::is_some(&pk_with_pop2_opt), 2);
        let pk_with_pop2 = option::extract(&mut pk_with_pop2_opt);

        let pks_with_pop = vector::empty<PublicKeyWithPoP>();
        vector::push_back(&mut pks_with_pop, pk_with_pop1);
        vector::push_back(&mut pks_with_pop, pk_with_pop2);

        let agg_pks: AggrPublicKeysWithPoP = bls12381::aggregate_pubkeys(pks_with_pop);
        let _ = agg_pks;
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
