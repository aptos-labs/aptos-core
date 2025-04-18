#[test_only]
module aptos_experimental::ca_transfer_example {
    use std::signer;
    use std::string::utf8;
    use aptos_std::debug::print;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_experimental::confidential_proof;
    use aptos_experimental::confidential_balance;
    use aptos_experimental::confidential_asset_tests;
    use aptos_experimental::confidential_asset;
    use aptos_experimental::twisted_elgamal;

    fun transfer(bob: &signer, alice: &signer, token: Object<Metadata>) {
        let bob_addr = signer::address_of(bob);
        let alice_addr = signer::address_of(alice);

        // It's a test-only function, so we don't need to worry about the security of the keypair.
        let (bob_dk, bob_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();
        let (alice_dk, alice_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();

        // Note: If the asset-specific auditor is set, we need to include it in the `auditor_eks` vector as the FIRST element.
        //
        // let asset_auditor_ek = confidential_asset::get_auditor(token);
        // let auditor_eks = vector[];
        // if (asset_auditor_ek.is_some()) {
        //     auditor_eks.push_back(asset_auditor_ek.extract());
        // };

        let (_, auditor_ek) = twisted_elgamal::generate_twisted_elgamal_keypair();
        let auditor_eks = vector[auditor_ek];

        let bob_ek_bytes = twisted_elgamal::pubkey_to_bytes(&bob_ek);
        let alice_ek_bytes = twisted_elgamal::pubkey_to_bytes(&alice_ek);

        confidential_asset::register(bob, token, bob_ek_bytes);
        confidential_asset::register(alice, token, alice_ek_bytes);

        // Bob's current balance is 300, and he wants to transfer 50 to Alice, whose balance is zero.
        let bob_current_amount = 300;
        let bob_new_amount = 250;
        let transfer_amount = 50;
        let alice_current_amount = 0;
        let alice_new_amount = 50;

        confidential_asset::deposit(bob, token, bob_current_amount);
        confidential_asset::rollover_pending_balance(bob, token);

        print(&utf8(b"Bob's actual balance is 300"));
        assert!(confidential_asset::verify_actual_balance(bob_addr, token, &bob_dk, (bob_current_amount as u128)));

        print(&utf8(b"Alice's pending balance is zero"));
        assert!(confidential_asset::verify_pending_balance(alice_addr, token, &alice_dk, alice_current_amount));

        let current_balance = confidential_balance::decompress_balance(&confidential_asset::actual_balance(bob_addr, token));

        let (
            proof,
            // New balance is the balance after the transfer encrypted with the sender's encryption key.
            // It will be set as the new actual balance for the sender.
            new_balance,
            // Transfer amount encrypted with the recipient's encryption key.
            // It will be Homomorphically added to the recipient's pending balance.
            transfer_amount,
            // Transfer amount encrypted with the auditors' encryption keys.
            // It won't be stored on-chain, but an auditor can decrypt the transfer amount with its dk.
            auditor_amounts
        ) = confidential_proof::prove_transfer(
            &bob_dk,
            &bob_ek,
            &alice_ek,
            transfer_amount,
            bob_new_amount,
            &current_balance,
            &auditor_eks,
        );

        let (
            sigma_proof,
            zkrp_new_balance,
            zkrp_transfer_amount
        ) = confidential_proof::serialize_transfer_proof(&proof);

        confidential_asset::confidential_transfer(
            bob,
            token,
            alice_addr,
            confidential_balance::balance_to_bytes(&new_balance),
            confidential_balance::balance_to_bytes(&transfer_amount),
            confidential_asset::serialize_auditor_eks(&auditor_eks),
            confidential_asset::serialize_auditor_amounts(&auditor_amounts),
            zkrp_new_balance,
            zkrp_transfer_amount,
            sigma_proof
        );

        print(&utf8(b"Bob's actual balance is 250"));
        assert!(confidential_asset::verify_actual_balance(bob_addr, token, &bob_dk, bob_new_amount));

        print(&utf8(b"Alice's pending balance is 50"));
        assert!(confidential_asset::verify_pending_balance(alice_addr, token, &alice_dk, alice_new_amount));
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
        alice: signer,
    ) {
        let token = confidential_asset_tests::set_up_for_confidential_asset_test(&confidential_asset, &aptos_fx, &fa, &bob, &bob, 500, 0);

        transfer(&bob, &alice, token);
    }
}
