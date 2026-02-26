#[test_only]
module confidential_asset_example::deposit_example {
    use std::signer;
    use std::string::utf8;
    use aptos_std::debug::print;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_framework::primary_fungible_store;

    use aptos_experimental::confidential_asset;
    use aptos_experimental::confidential_asset_tests;
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    fun deposit(bob: &signer, alice: &signer, token: Object<Metadata>) {
        let bob_addr = signer::address_of(bob);
        let alice_addr = signer::address_of(alice);

        // It's a test-only function, so we don't need to worry about the security of the keypair.
        let (bob_dk, bob_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();
        let (alice_dk, alice_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();

        let bob_proof = confidential_asset::prove_registration(bob_addr, token, &bob_dk);
        confidential_asset::register(bob, token, bob_ek, bob_proof);

        let alice_proof = confidential_asset::prove_registration(alice_addr, token, &alice_dk);
        confidential_asset::register(alice, token, alice_ek, alice_proof);

        print(&utf8(b"Bob's FA balance before the deposit is 300:"));
        print(&primary_fungible_store::balance(bob_addr, token));

        assert!(primary_fungible_store::balance(bob_addr, token) == 300);

        let bob_amount = 100;
        let alice_amount = 200;

        // The balance is not hidden yet, because we explicitly pass the amount to the function.
        // Each user deposits to their own confidential balance.
        confidential_asset::deposit(bob, token, bob_amount);
        confidential_asset::deposit(alice, token, alice_amount);

        print(&utf8(b"Bob's FA balance after the deposit is 200:"));
        print(&primary_fungible_store::balance(bob_addr, token));

        assert!(primary_fungible_store::balance(bob_addr, token) == 200);

        print(&utf8(b"Bob's pending balance is not zero:"));
        print(&confidential_asset::get_pending_balance(bob_addr, token));

        // In real world, we would not be able to see the someone else's balance as it requires
        // the knowledge of the decryption key.
        // The balance decryption requires solving the discrete logarithm problem,
        // so we just check if the passed amount is correct for simplicity.
        assert!(confidential_asset::check_pending_balance_decrypts_to(bob_addr, token, &bob_dk, bob_amount));

        print(&utf8(b"Alice's pending balance is not zero:"));
        print(&confidential_asset::get_pending_balance(alice_addr, token));

        assert!(confidential_asset::check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, alice_amount));
    }

    #[test(
        confidential_asset = @aptos_experimental,
        aptos_fx = @aptos_framework,
        fa = @0xfa,
        bob = @0xb0,
        alice = @0xa1
    )]
    fun deposit_example_test(
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
            &alice,
            300,
            200
        );

        deposit(&bob, &alice, token);
    }
}
