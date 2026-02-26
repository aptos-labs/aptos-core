#[test_only]
module confidential_asset_example::transfer_example {
    use std::signer;
    use std::string::utf8;
    use aptos_std::debug::print;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;

    use aptos_experimental::confidential_asset;
    use aptos_experimental::confidential_asset_tests;
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    fun transfer(bob: &signer, alice: &signer, token: Object<Metadata>) {
        let bob_addr = signer::address_of(bob);
        let alice_addr = signer::address_of(alice);

        // It's a test-only function, so we don't need to worry about the security of the keypair.
        let (bob_dk, bob_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();
        let (alice_dk, alice_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();

        let bob_proof = confidential_asset::prove_registration(bob_addr, token, &bob_dk);
        confidential_asset::register(bob, token, bob_ek, bob_proof);

        let alice_proof = confidential_asset::prove_registration(alice_addr, token, &alice_dk);
        confidential_asset::register(alice, token, alice_ek, alice_proof);

        // Bob's current balance is 300, and he wants to transfer 50 to Alice, whose balance is zero.
        let bob_current_amount: u128 = 300;
        let bob_new_amount: u128 = 250;
        let transfer_amount: u64 = 50;
        let alice_new_amount: u64 = 50;

        confidential_asset::deposit(bob, token, (bob_current_amount as u64));
        confidential_asset::rollover_pending_balance(bob, token);

        print(&utf8(b"Bob's actual balance is 300"));
        assert!(confidential_asset::check_available_balance_decrypts_to(bob_addr, token, &bob_dk, bob_current_amount));

        print(&utf8(b"Alice's pending balance is zero"));
        assert!(confidential_asset::check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, 0));

        let (proof, _test_auditor_amounts) = confidential_asset::prove_transfer(
            bob_addr,
            alice_addr,
            token,
            &bob_dk,
            transfer_amount,
            bob_new_amount,
            &vector[], // no extra auditors
        );

        confidential_asset::confidential_transfer(
            bob,
            token,
            alice_addr,
            vector[], // no extra auditor EKs
            proof
        );

        print(&utf8(b"Bob's actual balance is 250"));
        assert!(confidential_asset::check_available_balance_decrypts_to(bob_addr, token, &bob_dk, bob_new_amount));

        print(&utf8(b"Alice's pending balance is 50"));
        assert!(confidential_asset::check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, alice_new_amount));
    }

    #[test(
        confidential_asset = @aptos_experimental,
        aptos_fx = @aptos_framework,
        fa = @0xfa,
        bob = @0xb0,
        alice = @0xa1
    )]
    fun transfer_example_test(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        bob: signer,
        alice: signer)
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

        transfer(&bob, &alice, token);
    }
}
