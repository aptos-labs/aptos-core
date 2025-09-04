module poc::public_key_validate_internal {
    use velor_std::ed25519::{Self};
    use std::option::{Self};
    use std::vector;

    public entry fun main(_owner: &signer) {
        let valid_pk_bytes = x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let result_ok = ed25519::new_validated_public_key_from_bytes(valid_pk_bytes);
        assert!(option::is_some(&result_ok), 1);
        let _ = result_ok;

        let invalid_pk_bytes_zeros = x"0000000000000000000000000000000000000000000000000000000000000000";
        let result_fail_zeros = ed25519::new_validated_public_key_from_bytes(invalid_pk_bytes_zeros);
        assert!(option::is_none(&result_fail_zeros), 2);

        let invalid_len_31 = vector::empty<u8>();
        let i = 0; while (i < 31) { vector::push_back(&mut invalid_len_31, 0u8); i = i + 1; };
        let result_fail_len31 = ed25519::new_validated_public_key_from_bytes(invalid_len_31);
        assert!(option::is_none(&result_fail_len31), 3);

        let invalid_len_33 = vector::empty<u8>();
        let i = 0; while (i < 33) { vector::push_back(&mut invalid_len_33, 0u8); i = i + 1; };
        let result_fail_len33 = ed25519::new_validated_public_key_from_bytes(invalid_len_33);
        assert!(option::is_none(&result_fail_len33), 4);

        let invalid_pk_bytes_high = x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let result_fail_high = ed25519::new_validated_public_key_from_bytes(invalid_pk_bytes_high);
        assert!(option::is_none(&result_fail_high), 5);

        let small_order_pk_bytes = x"0100000000000000000000000000000000000000000000000000000000000000";
        let result_fail_small_order = ed25519::new_validated_public_key_from_bytes(small_order_pk_bytes);
        assert!(option::is_none(&result_fail_small_order), 6);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
