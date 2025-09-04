#[test_only]
module confidential_asset_example::deposit_example {
    use std::signer;
    use std::string::utf8;
    use velor_std::debug::print;
    use velor_framework::fungible_asset::Metadata;
    use velor_framework::object::Object;
    use velor_framework::primary_fungible_store;

    use velor_experimental::confidential_asset;
    use velor_experimental::confidential_asset_tests;
    use velor_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    fun deposit(bob: &signer, alice: &signer, token: Object<Metadata>) {
        let bob_addr = signer::address_of(bob);
        let alice_addr = signer::address_of(alice);

        // It's a test-only function, so we don't need to worry about the security of the keypair.
        let (bob_dk, bob_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();
        let (alice_dk, alice_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();

        let bob_ek = twisted_elgamal::pubkey_to_bytes(&bob_ek);
        let alice_ek = twisted_elgamal::pubkey_to_bytes(&alice_ek);

        confidential_asset::register(bob, token, bob_ek);
        confidential_asset::register(alice, token, alice_ek);

        print(&utf8(b"Bob's FA balance before the deposit is 500:"));
        print(&primary_fungible_store::balance(bob_addr, token));

        assert!(primary_fungible_store::balance(bob_addr, token) == 500);

        let bob_amount = 100;
        let alice_amount = 200;

        // The balance is not hidden yet, because we explicitly pass the amount to the function.
        confidential_asset::deposit(bob, token, bob_amount);
        confidential_asset::deposit_to(bob, token, alice_addr, alice_amount);

        print(&utf8(b"Bob's FA balance after the deposit is 200:"));
        print(&primary_fungible_store::balance(bob_addr, token));

        assert!(primary_fungible_store::balance(bob_addr, token) == 200);

        print(&utf8(b"Bob's pending balance is not zero:"));
        print(&confidential_asset::pending_balance(bob_addr, token));

        // In real world, we would not be able to see the someone else's balance as it requires
        // the knowledge of the decryption key.
        // The balance decryption requires solving the discrete logarithm problem,
        // so we just check if the passed amount is correct for simplicity.
        assert!(confidential_asset::verify_pending_balance(bob_addr, token, &bob_dk, bob_amount));

        print(&utf8(b"Alice's pending balance is not zero:"));
        print(&confidential_asset::pending_balance(alice_addr, token));

        assert!(confidential_asset::verify_pending_balance(alice_addr, token, &alice_dk, alice_amount));
    }

    #[test(
        confidential_asset = @velor_experimental,
        velor_fx = @velor_framework,
        fa = @0xfa,
        bob = @0xb0,
        alice = @0xa1
    )]
    fun deposit_example_test(
        confidential_asset: signer,
        velor_fx: signer,
        fa: signer,
        bob: signer,
        alice: signer)
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

        deposit(&bob, &alice, token);
    }
}
