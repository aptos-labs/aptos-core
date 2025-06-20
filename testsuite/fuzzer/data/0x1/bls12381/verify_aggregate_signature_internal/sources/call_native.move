module poc::verify_aggregate_signature_internal {
    use aptos_std::bls12381::{Self, PublicKeyWithPoP, ProofOfPossession};
    use std::vector;
    use std::option::{Self};

    fun create_pks(pks_bytes: &vector<vector<u8>>, pops: &vector<ProofOfPossession>): vector<PublicKeyWithPoP> {
        let pks_structs = vector::empty<PublicKeyWithPoP>();
        let i = 0;
        while (i < vector::length(pks_bytes)) {
            let pk_opt = bls12381::public_key_from_bytes_with_pop(*vector::borrow(pks_bytes, i), vector::borrow(pops, i));
            assert!(option::is_some(&pk_opt), (i + 1) as u64);
            vector::push_back(&mut pks_structs, option::extract(&mut pk_opt));
            i = i + 1;
        };
        pks_structs
    }

    public entry fun main(_owner: &signer) {
        let pk_bytes_vec = vector[
            x"808864c91ae7a9998b3f5ee71f447840864e56d79838e4785ff5126c51480198df3d972e1e0348c6da80d396983e42d7",
            x"8843843c76d167c02842a214c21277bad0bfd83da467cb5cf2d3ee67b2dcc7221b9fafa6d430400164012580e0c34d27"
        ];
        let pop_bytes_vec = vector[
            x"ab42afff92510034bf1232a37a0d31bc8abfc17e7ead9170d2d100f6cf6c75ccdcfedbd31699a112b4464a06fd636f3f190595863677d660b4c5d922268ace421f9e86e3a054946ee34ce29e1f88c1a10f27587cf5ec528d65ba7c0dc4863364",
            x"a6da5f2bc17df70ce664cff3e3a3e09d17162e47e652032b9fedc0c772fd5a533583242cba12095602e422e579c5284b1735009332dbdd23430bbcf61cc506ae37e41ff9a1fc78f0bc0d99b6bc7bf74c8f567dfb59079a035842bdc5fa3a0464"
        ];
        let messages_vec = vector[b"hello", b"world"];
        let invalid_agg_sig_bytes = x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

        let pops_structs = vector::empty<ProofOfPossession>();
        vector::push_back(&mut pops_structs, bls12381::proof_of_possession_from_bytes(pop_bytes_vec[0]));
        vector::push_back(&mut pops_structs, bls12381::proof_of_possession_from_bytes(pop_bytes_vec[1]));

        let pks_structs = create_pks(&pk_bytes_vec, &pops_structs);
        let invalid_agg_sig_struct = bls12381::aggr_or_multi_signature_from_bytes(invalid_agg_sig_bytes);

        let result_fail = bls12381::verify_aggregate_signature(&invalid_agg_sig_struct, pks_structs, messages_vec);
        assert!(!result_fail, 101);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
