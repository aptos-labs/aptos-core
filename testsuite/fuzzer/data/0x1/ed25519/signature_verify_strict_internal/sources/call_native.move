module poc::signature_verify_strict_internal {
    use velor_std::ed25519::{Self};

    public entry fun main(_owner: &signer) {
        let pk_bytes = x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let message = b"message to sign";
        let invalid_sig_bytes = x"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

        let pk = ed25519::new_unvalidated_public_key_from_bytes(pk_bytes);
        let invalid_sig = ed25519::new_signature_from_bytes(invalid_sig_bytes);

        let result_fail = ed25519::signature_verify_strict(&invalid_sig, &pk, message);
        assert!(!result_fail, 1);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
