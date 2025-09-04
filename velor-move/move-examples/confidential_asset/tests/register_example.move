#[test_only]
module confidential_asset_example::register_example {
    use std::signer;
    use std::string::utf8;
    use velor_std::debug::print;
    use velor_framework::fungible_asset::Metadata;
    use velor_framework::object::Object;

    use velor_experimental::confidential_asset;
    use velor_experimental::confidential_asset_tests;
    use velor_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    fun register(bob: &signer, token: Object<Metadata>) {
        let bob_addr = signer::address_of(bob);

        // It's a test-only function, so we don't need to worry about the security of the keypair.
        let (_bob_dk, bob_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();

        let bob_ek = twisted_elgamal::pubkey_to_bytes(&bob_ek);

        confidential_asset::register(bob, token, bob_ek);

        print(&utf8(b"Bob's pending balance is zero:"));
        print(&confidential_asset::pending_balance(bob_addr, token));

        print(&utf8(b"Bob's actual balance is zero:"));
        print(&confidential_asset::actual_balance(bob_addr, token));

        print(&utf8(b"Bob's encryption key is set:"));
        print(&confidential_asset::encryption_key(bob_addr, token));
    }

    #[test(
        confidential_asset = @velor_experimental,
        velor_fx = @velor_framework,
        fa = @0xfa,
        bob = @0xb0
    )]
    fun register_example_test(
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
            0,
            0
        );

        register(&bob, token);
    }
}
