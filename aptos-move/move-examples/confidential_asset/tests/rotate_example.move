#[test_only]
module confidential_asset_example::rotate_example {
    use std::signer;
    use std::string::utf8;
    use aptos_std::debug::print;
    use aptos_std::ristretto255::basepoint_H;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;

    use aptos_experimental::confidential_asset;
    use aptos_experimental::confidential_asset_tests;
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    fun rotate(bob: &signer, token: Object<Metadata>) {
        let bob_addr = signer::address_of(bob);

        // It's a test-only function, so we don't need to worry about the security of the keypair.
        let (bob_current_dk, bob_current_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();
        let (bob_new_dk, _bob_new_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();

        let bob_amount: u128 = 100;

        let reg_proof = confidential_asset::prove_registration(bob_addr, token, &bob_current_dk);
        confidential_asset::register(bob, token, bob_current_ek, reg_proof);
        confidential_asset::deposit(bob, token, (bob_amount as u64));

        // We need to rollover the pending balance and pause incoming transfers
        // to prevent any new deposits from coming in during rotation.
        confidential_asset::rollover_pending_balance_and_pause(bob, token);

        print(&utf8(b"Bob's encryption key before the rotation:"));
        print(&confidential_asset::get_encryption_key(bob_addr, token));

        assert!(confidential_asset::check_available_balance_decrypts_to(bob_addr, token, &bob_current_dk, bob_amount));

        // Compute the new EK from the new DK: ek = (1/dk) * H
        let new_ek = basepoint_H().point_mul(
            &bob_new_dk.scalar_invert().extract()
        );

        let rotation_proof = confidential_asset::prove_key_rotation(
            bob_addr,
            token,
            &bob_current_dk,
            &bob_new_dk,
        );

        // After rotating the encryption key, we unpause incoming transfers.
        confidential_asset::rotate_encryption_key(
            bob,
            token,
            new_ek,
            rotation_proof,
            true, // resume_incoming_transfers
        );

        print(&utf8(b"Bob's encryption key after the rotation:"));
        print(&confidential_asset::get_encryption_key(bob_addr, token));

        // Note that here we use the new decryption key to verify the actual balance.
        assert!(confidential_asset::check_available_balance_decrypts_to(bob_addr, token, &bob_new_dk, bob_amount));
    }

    #[test(
        confidential_asset = @aptos_experimental,
        aptos_fx = @aptos_framework,
        fa = @0xfa,
        bob = @0xb0
    )]
    fun rotate_example_test(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        bob: signer)
    {
        let token = confidential_asset_tests::set_up_for_confidential_asset_test(
            &confidential_asset,
            &aptos_fx,
            &fa,
            &bob,
            &bob,
            500,
            0
        );

        rotate(&bob, token);
    }
}
