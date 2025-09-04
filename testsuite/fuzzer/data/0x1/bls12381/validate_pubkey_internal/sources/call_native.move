module poc::validate_pubkey_internal {
    use velor_std::bls12381;
    use std::option::{Self, Option};

    public entry fun main(_owner: &signer) {
        let valid_pk_bytes = x"94209a296b739577cb076d3bfb1ca8ee936f29b69b7dae436118c4dd1cc26fd43dcd16249476a006b8b949bf022a7858";
        let maybe_pk: Option<bls12381::PublicKey> = bls12381::public_key_from_bytes(valid_pk_bytes);
        assert!(option::is_some(&maybe_pk), 0);

        let invalid_pk_bytes_short = x"00";
        let maybe_pk_invalid_short = bls12381::public_key_from_bytes(invalid_pk_bytes_short);
        assert!(option::is_none(&maybe_pk_invalid_short), 1);

        let invalid_pk_bytes_bad = x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let maybe_pk_invalid_bad = bls12381::public_key_from_bytes(invalid_pk_bytes_bad);
        assert!(option::is_none(&maybe_pk_invalid_bad), 2);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
