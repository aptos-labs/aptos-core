#[test_only]
module confidential_asset_example::normalize_example {
    use std::signer;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;

    use aptos_experimental::confidential_asset;
    use aptos_experimental::confidential_asset_tests;
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    fun normalize(bob: &signer, token: Object<Metadata>) {
        let bob_addr = signer::address_of(bob);

        // It's a test-only function, so we don't need to worry about the security of the keypair.
        let (bob_dk, bob_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();

        let bob_amount: u128 = 500;

        let proof = confidential_asset::prove_registration(bob_addr, token, &bob_dk);
        confidential_asset::register(bob, token, bob_ek, proof);
        confidential_asset::deposit(bob, token, (bob_amount as u64));

        // The rollover function is the only function that requires the actual balance to be normalized
        // beforehand and leaves it unnormalized after execution, no matter what the pending balance was.
        confidential_asset::rollover_pending_balance(bob, token);

        assert!(!confidential_asset::is_normalized(bob_addr, token));

        confidential_asset::deposit(bob, token, (bob_amount as u64));

        // Before performing a second rollover, the actual balance must be normalized.
        // You will get an error if you try to rollover an unnormalized balance:
        // confidential_asset::rollover_pending_balance(bob, token);

        let normalization_proof = confidential_asset::prove_normalization(
            bob_addr,
            token,
            &bob_dk,
            bob_amount,
        );

        confidential_asset::normalize(
            bob,
            token,
            normalization_proof
        );

        assert!(confidential_asset::is_normalized(bob_addr, token));
        assert!(confidential_asset::check_available_balance_decrypts_to(bob_addr, token, &bob_dk, bob_amount));

        // A rollover can be performed once the balance is normalized.
        // Note that functions like `withdraw` and `confidential_transfer` do not require the actual balance
        // to be normalized beforehand, as zk-proofs guarantee that the actual balance is normalized after
        // their execution.
        confidential_asset::rollover_pending_balance(bob, token);
    }

    #[test(
        confidential_asset = @aptos_experimental,
        aptos_fx = @aptos_framework,
        fa = @0xfa,
        bob = @0xb0
    )]
    fun normalize_example_test(
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
            1000,
            0
        );

        normalize(&bob, token);
    }
}
