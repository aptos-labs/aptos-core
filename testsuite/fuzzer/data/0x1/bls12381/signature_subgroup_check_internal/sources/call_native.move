module poc::signature_subgroup_check_internal {
    use aptos_std::bls12381::{Self, Signature};

    public entry fun main(_owner: &signer) {
        let valid_sig_bytes = x"b01ce4632e94d8c611736e96aa2ad8e0528a02f927a81a92db8047b002a8c71dc2d6bfb94729d0973790c10b6ece446817e4b7543afd7ca9a17c75de301ae835d66231c26a003f11ae26802b98d90869a9e73788c38739f7ac9d52659e1f7cf7";
        let sig: Signature = bls12381::signature_from_bytes(valid_sig_bytes);
        assert!(bls12381::signature_subgroup_check(&sig), 0);

        let invalid_sig_bytes_identity = x"c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let sig_invalid: Signature = bls12381::signature_from_bytes(invalid_sig_bytes_identity);
        assert!(!bls12381::signature_subgroup_check(&sig_invalid), 1);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
