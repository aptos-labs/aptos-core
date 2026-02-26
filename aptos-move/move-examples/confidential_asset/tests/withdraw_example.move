#[test_only]
module confidential_asset_example::withdraw_example {
    use std::signer;
    use std::string::utf8;
    use aptos_std::debug::print;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_framework::primary_fungible_store;

    use aptos_experimental::confidential_asset;
    use aptos_experimental::confidential_asset_tests;
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    fun withdraw(bob: &signer, alice: &signer, token: Object<Metadata>) {
        let bob_addr = signer::address_of(bob);
        let alice_addr = signer::address_of(alice);

        // It's a test-only function, so we don't need to worry about the security of the keypair.
        let (bob_dk, bob_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();
        let (alice_dk, alice_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();

        let bob_proof = confidential_asset::prove_registration(bob_addr, token, &bob_dk);
        confidential_asset::register(bob, token, bob_ek, bob_proof);

        let alice_proof = confidential_asset::prove_registration(alice_addr, token, &alice_dk);
        confidential_asset::register(alice, token, alice_ek, alice_proof);

        let bob_current_amount: u128 = 500;
        let bob_new_amount: u128 = 450;
        let withdraw_amount: u64 = 50;

        // Bob deposits and rolls over so funds are in his available balance.
        confidential_asset::deposit(bob, token, (bob_current_amount as u64));
        confidential_asset::rollover_pending_balance(bob, token);

        print(&utf8(b"Alice's FA balance before the withdrawal is zero:"));
        print(&primary_fungible_store::balance(alice_addr, token));
        assert!(primary_fungible_store::balance(alice_addr, token) == 0);

        print(&utf8(b"Bob's actual balance before the withdrawal is 500"));
        assert!(confidential_asset::check_available_balance_decrypts_to(bob_addr, token, &bob_dk, bob_current_amount));

        let proof = confidential_asset::prove_withdrawal(
            bob_addr,
            token,
            &bob_dk,
            withdraw_amount,
            bob_new_amount,
        );

        // Bob withdraws 50 tokens to Alice's FA store (not confidential balance).
        confidential_asset::withdraw_to(
            bob,
            token,
            alice_addr,
            withdraw_amount,
            proof
        );

        print(&utf8(b"Alice's FA balance after the withdrawal is 50:"));
        print(&primary_fungible_store::balance(alice_addr, token));
        assert!(primary_fungible_store::balance(alice_addr, token) == 50);

        print(&utf8(b"Bob's actual balance after the withdrawal is 450"));
        assert!(confidential_asset::check_available_balance_decrypts_to(bob_addr, token, &bob_dk, bob_new_amount));
    }

    #[test(
        confidential_asset = @aptos_experimental,
        aptos_fx = @aptos_framework,
        fa = @0xfa,
        bob = @0xb0,
        alice = @0xa1
    )]
    fun withdraw_example_test(
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

        withdraw(&bob, &alice, token);
    }
}
