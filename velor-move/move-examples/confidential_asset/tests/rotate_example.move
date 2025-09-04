#[test_only]
module confidential_asset_example::rotate_example {
    use std::signer;
    use std::string::utf8;
    use velor_std::debug::print;
    use velor_framework::fungible_asset::Metadata;
    use velor_framework::object::Object;

    use velor_experimental::confidential_asset;
    use velor_experimental::confidential_asset_tests;
    use velor_experimental::confidential_balance;
    use velor_experimental::confidential_proof;
    use velor_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    fun rotate(bob: &signer, token: Object<Metadata>) {
        let bob_addr = signer::address_of(bob);

        // It's a test-only function, so we don't need to worry about the security of the keypair.
        let (bob_current_dk, bob_current_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();
        let (bob_new_dk, bob_new_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();

        let bob_current_ek_bytes = twisted_elgamal::pubkey_to_bytes(&bob_current_ek);
        let bob_new_ek_bytes = twisted_elgamal::pubkey_to_bytes(&bob_new_ek);

        let bob_amount = 100;

        confidential_asset::register(bob, token, bob_current_ek_bytes);
        confidential_asset::deposit(bob, token, (bob_amount as u64));

        // We need to rollover the pending balance and freeze the token to prevent any new deposits being come.
        confidential_asset::rollover_pending_balance_and_freeze(bob, token);

        print(&utf8(b"Bob's encryption key before the rotation:"));
        print(&confidential_asset::encryption_key(bob_addr, token));

        assert!(confidential_asset::verify_actual_balance(bob_addr, token, &bob_current_dk, bob_amount));

        let current_balance = confidential_balance::decompress_balance(
            &confidential_asset::actual_balance(bob_addr, token)
        );

        let (proof, new_balance) = confidential_proof::prove_rotation(
            &bob_current_dk,
            &bob_new_dk,
            &bob_current_ek,
            &bob_new_ek,
            bob_amount,
            &current_balance
        );

        let (
            sigma_proof,
            zkrp_new_balance
        ) = confidential_proof::serialize_rotation_proof(&proof);

        // After rotating the encryption key, we unfreeze the token to allow new deposits.
        confidential_asset::rotate_encryption_key_and_unfreeze(
            bob,
            token,
            bob_new_ek_bytes,
            confidential_balance::balance_to_bytes(&new_balance),
            zkrp_new_balance,
            sigma_proof
        );

        print(&utf8(b"Bob's encryption key after the rotation:"));
        print(&confidential_asset::encryption_key(bob_addr, token));

        // Note that here we use the new decryption key to verify the actual balance.
        assert!(confidential_asset::verify_actual_balance(bob_addr, token, &bob_new_dk, bob_amount));
    }

    #[test(
        confidential_asset = @velor_experimental,
        velor_fx = @velor_framework,
        fa = @0xfa,
        bob = @0xb0
    )]
    fun rotate_example_test(
        confidential_asset: signer,
        velor_fx: signer,
        fa: signer,
        bob: signer)
    {
        let token = confidential_asset_tests::set_up_for_confidential_asset_test(
            &confidential_asset,
            &velor_fx,
            &fa,
            &bob,
            &bob,
            500,
            0
        );

        rotate(&bob, token);
    }
}
