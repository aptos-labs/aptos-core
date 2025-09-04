module poc::verify_normal_signature_internal {
    use velor_std::bls12381::{Self};
    use std::option::{Self};

    public entry fun main(_owner: &signer) {
        let pk_bytes = x"94209a296b739577cb076d3bfb1ca8ee936f29b69b7dae436118c4dd1cc26fd43dcd16249476a006b8b949bf022a7858";
        let message = b"some message";
        let invalid_sig_bytes = x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

        let maybe_pk = bls12381::public_key_from_bytes(pk_bytes);
        assert!(option::is_some(&maybe_pk), 1);
        let pk = option::extract(&mut maybe_pk);

        let invalid_sig = bls12381::signature_from_bytes(invalid_sig_bytes);

        let result_fail = bls12381::verify_normal_signature(&invalid_sig, &pk, message);
        assert!(!result_fail, 101);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
