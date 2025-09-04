module poc::point_is_canonical_internal {
    use velor_std::ristretto255::{Self};
    use std::option::{Self};

    public entry fun main(_owner: &signer) {
        let canonical_bytes = x"e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76";
        let maybe_point_ok = ristretto255::new_compressed_point_from_bytes(canonical_bytes);
        assert!(option::is_some(&maybe_point_ok), 1);

        let non_canonical_bytes = x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let maybe_point_fail = ristretto255::new_compressed_point_from_bytes(non_canonical_bytes);
        assert!(option::is_none(&maybe_point_fail), 2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
