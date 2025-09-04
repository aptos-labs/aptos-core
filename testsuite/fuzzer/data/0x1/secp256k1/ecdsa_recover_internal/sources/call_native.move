module poc::ecdsa_recover_internal {
    use velor_std::secp256k1;
    use std::option;
    use std::hash;

    public entry fun main() {
        let msg = b"test velor secp256k1";
        let h = hash::sha2_256(msg);
        let sig = secp256k1::ecdsa_signature_from_bytes(x"f7ad936da03f948c14c542020e3c5f4e02aaacd1f20427c11aa6e2fbf8776477646bba0e1a37f9e7c777c423a1d2849baafd7ff6a9930814a43c3f80d59db56f");
        let pk_opt = secp256k1::ecdsa_recover(h, 0, &sig);
        assert!(option::is_some(&pk_opt), 1);
        let pk = option::extract(&mut pk_opt);
        let pk_bytes = secp256k1::ecdsa_raw_public_key_to_bytes(&pk);
        assert!(pk_bytes == x"4646ae5047316b4230d0086c8acec687f00b1cd9d1dc634f6cb358ac0a9a8ffffe77b4dd0a4bfb95851f3b7355c781dd60f8418fc8a65d14907aff47c903a559", 2);
    }

    #[test]
    fun a() {
        main()
    }
}
