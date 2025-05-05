module poc::public_key_validate_v2_internal {
    use aptos_std::multi_ed25519::{Self, ValidatedPublicKey};
    use std::option::Option;

    public entry fun main(_owner: &signer) {
        let bytes = vector<u8>[1u8];
        let _maybe_pk: Option<ValidatedPublicKey> = multi_ed25519::new_validated_public_key_from_bytes_v2(bytes);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
