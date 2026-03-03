#[test_only]
module aptos_experimental::confidential_asset_tests {
    use std::features;
    use std::option;
    use std::signer;
    use std::string::utf8;
    use aptos_std::ristretto255::{Scalar, CompressedRistretto};
    use aptos_framework::account;
    use aptos_framework::chain_id;
    use aptos_framework::coin;
    use aptos_framework::fungible_asset::{Self, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;

    use aptos_experimental::confidential_asset;
    use aptos_experimental::confidential_balance::{Pending, Balance};
    use aptos_experimental::ristretto255_twisted_elgamal::generate_twisted_elgamal_keypair;

    struct MockCoin {}

    /// Registers a user with a real sigma protocol proof.
    fun register(
        sender: &signer,
        dk: &Scalar,
        ek: CompressedRistretto,
        token: Object<Metadata>,
    ) {
        let proof = confidential_asset::prove_registration(signer::address_of(sender), token, dk);
        confidential_asset::register(sender, token, ek, proof);
    }

    fun withdraw(
        sender: &signer,
        sender_dk: &Scalar,
        token: Object<Metadata>,
        to: address,
        amount: u64,
        new_amount: u128
    ) {
        let proof = confidential_asset::prove_withdrawal(
            signer::address_of(sender),
            token,
            sender_dk,
            amount,
            new_amount,
        );

        confidential_asset::withdraw_to(
            sender,
            token,
            to,
            amount,
            proof
        );
    }

    fun transfer(
        sender: &signer,
        sender_dk: &Scalar,
        token: Object<Metadata>,
        to: address,
        amount: u64,
        new_amount: u128
    ) {
        let proof = confidential_asset::prove_transfer(
            signer::address_of(sender), to, token, sender_dk, amount, new_amount, &vector[],
        );

        confidential_asset::confidential_transfer(sender, token, to, proof);
    }

    fun audit_transfer(
        sender: &signer,
        sender_dk: &Scalar,
        token: Object<Metadata>,
        to: address,
        amount: u64,
        new_amount: u128,
        volun_auditor_eks: &vector<CompressedRistretto>,
    ): (Balance<Pending>, vector<Balance<Pending>>) {
        let proof = confidential_asset::prove_transfer(
            signer::address_of(sender), to, token, sender_dk, amount, new_amount, volun_auditor_eks,
        );

        let eff_aud_amount = confidential_asset::get_amount_ciphertext_for_effective_auditor(&proof);
        let volun_aud_amounts = confidential_asset::get_amount_ciphertexts_for_volun_auditors(&proof);

        confidential_asset::confidential_transfer(sender, token, to, proof);

        (eff_aud_amount, volun_aud_amounts)
    }

    fun rotate(
        sender: &signer,
        sender_dk: &Scalar,
        token: Object<Metadata>,
        new_dk: &Scalar,
    ) {
        let proof =
            confidential_asset::prove_key_rotation(
                signer::address_of(sender),
                token,
                sender_dk,
                new_dk,
            );

        confidential_asset::rotate_encryption_key(
            sender,
            token,
            proof,
            true, // unpause
        );
    }

    public fun set_up_for_confidential_asset_test(
        confidential_asset: &signer,
        aptos_fx: &signer,
        fa: &signer,
        sender: &signer,
        recipient: &signer,
        sender_amount: u64,
        recipient_amount: u64
    ): Object<Metadata> {
        chain_id::initialize_for_test(aptos_fx, 4);

        let ctor_ref = &object::create_sticky_object(signer::address_of(fa));

        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            ctor_ref,
            option::none(),
            utf8(b"MockToken"),
            utf8(b"MT"),
            18,
            utf8(b"https://"),
            utf8(b"https://")
        );

        let mint_ref = fungible_asset::generate_mint_ref(ctor_ref);

        assert!(signer::address_of(aptos_fx) != signer::address_of(sender), 1);
        assert!(signer::address_of(aptos_fx) != signer::address_of(recipient), 2);

        confidential_asset::init_module_for_testing(confidential_asset);

        features::change_feature_flags_for_testing(
            aptos_fx,
            vector[features::get_bulletproofs_feature()],
            vector[]
        );

        let token = object::object_from_constructor_ref<Metadata>(ctor_ref);

        let sender_store =
            primary_fungible_store::ensure_primary_store_exists(
                signer::address_of(sender), token
            );
        fungible_asset::mint_to(&mint_ref, sender_store, sender_amount);

        let recipient_store =
            primary_fungible_store::ensure_primary_store_exists(
                signer::address_of(recipient), token
            );
        fungible_asset::mint_to(&mint_ref, recipient_store, recipient_amount);

        token
    }

    /// Helper: tests confidential_transfer with various auditor configurations.
    fun test_confidential_transfer_impl(
        confidential_asset: &signer,
        aptos_fx: &signer,
        fa: &signer,
        alice: &signer,
        bob: &signer,
        has_eff_auditor: bool,
        num_volun_auditors: u64,
    ) {
        let token = set_up_for_confidential_asset_test(
            confidential_asset, aptos_fx, fa, alice, bob, 500, 500,
        );

        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();
        let (bob_dk, bob_ek) = generate_twisted_elgamal_keypair();

        let eff_aud_dk = if (has_eff_auditor) {
            let (dk, ek) = generate_twisted_elgamal_keypair();
            confidential_asset::set_auditor_for_asset_type(
                aptos_fx, token, option::some(ek.compressed_point_to_bytes()),
            );
            option::some(dk)
        } else {
            option::none<Scalar>()
        };

        let volun_aud_dks = vector[];
        let volun_aud_eks = vector[];
        let i = 0;
        while (i < num_volun_auditors) {
            let (dk, ek) = generate_twisted_elgamal_keypair();
            volun_aud_dks.push_back(dk);
            volun_aud_eks.push_back(ek);
            i = i + 1;
        };

        register(alice, &alice_dk, alice_ek, token);
        register(bob, &bob_dk, bob_ek, token);

        confidential_asset::deposit(alice, token, 200);
        confidential_asset::rollover_pending_balance(alice, token);

        let proof = confidential_asset::prove_transfer(
            alice_addr, bob_addr, token, &alice_dk, 100, 100, &volun_aud_eks,
        );

        if (has_eff_auditor) {
            let eff_aud_amount = confidential_asset::get_amount_ciphertext_for_effective_auditor(&proof);
            assert!(eff_aud_amount.check_decrypts_to(eff_aud_amount.get_R(), option::borrow(&eff_aud_dk), 100), 1);
        };

        if (num_volun_auditors > 0) {
            let volun_aud_amounts = confidential_asset::get_amount_ciphertexts_for_volun_auditors(&proof);
            let i = 0;
            while (i < num_volun_auditors) {
                assert!(volun_aud_amounts[i].check_decrypts_to(volun_aud_amounts[i].get_R(), &volun_aud_dks[i], 100), 1);
                i = i + 1;
            };
        };

        confidential_asset::confidential_transfer(alice, token, bob_addr, proof);

        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 100, false),
            1
        );
        assert!(
            confidential_asset::check_pending_balance_decrypts_to(bob_addr, token, &bob_dk, 100),
            1
        );

        if (has_eff_auditor) {
            assert!(
                confidential_asset::check_available_balance_decrypts_to(
                    alice_addr, token, option::borrow(&eff_aud_dk), 100, true,
                ),
                1
            );
        };
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_register_test(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer,
        bob: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset,
                &aptos_fx,
                &fa,
                &alice,
                &bob,
                0,
                0
            );

        let alice_addr = signer::address_of(&alice);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();

        register(&alice, &alice_dk, alice_ek, token);

        assert!(
            confidential_asset::check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, 0),
            1
        );
        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 0, false),
            1
        );
        assert!(confidential_asset::get_encryption_key(alice_addr, token) == alice_ek, 1);
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_rollover_test(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer,
        bob: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset,
                &aptos_fx,
                &fa,
                &alice,
                &bob,
                500,
                0
            );

        let alice_addr = signer::address_of(&alice);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();

        register(&alice, &alice_dk, alice_ek, token);
        confidential_asset::deposit(&alice, token, 100);

        assert!(
            confidential_asset::check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, 100),
            1
        );
        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 0, false),
            1
        );
        assert!(confidential_asset::is_normalized(alice_addr, token));

        confidential_asset::rollover_pending_balance(&alice, token);

        assert!(
            confidential_asset::check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, 0),
            1
        );
        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 100, false),
            1
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_deposit_test(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer,
        bob: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset,
                &aptos_fx,
                &fa,
                &alice,
                &bob,
                500,
                500
            );

        let alice_addr = signer::address_of(&alice);
        let bob_addr = signer::address_of(&bob);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();
        let (bob_dk, bob_ek) = generate_twisted_elgamal_keypair();

        register(&alice, &alice_dk, alice_ek, token);
        register(&bob, &bob_dk, bob_ek, token);

        confidential_asset::deposit(&alice, token, 100);
        confidential_asset::deposit(&bob, token, 150);

        assert!(primary_fungible_store::balance(alice_addr, token) == 400, 1);
        assert!(
            confidential_asset::check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, 100),
            1
        );
        assert!(
            confidential_asset::check_pending_balance_decrypts_to(bob_addr, token, &bob_dk, 150),
            1
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_withdraw_test(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer,
        bob: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset,
                &aptos_fx,
                &fa,
                &alice,
                &bob,
                500,
                500
            );

        let alice_addr = signer::address_of(&alice);
        let bob_addr = signer::address_of(&bob);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();

        register(&alice, &alice_dk, alice_ek, token);

        confidential_asset::deposit(&alice, token, 200);
        confidential_asset::rollover_pending_balance(&alice, token);

        withdraw(&alice, &alice_dk, token, bob_addr, 50, 150);

        assert!(primary_fungible_store::balance(bob_addr, token) == 550, 1);
        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 150, false),
            1
        );

        withdraw(&alice, &alice_dk, token, alice_addr, 50, 100);

        assert!(primary_fungible_store::balance(alice_addr, token) == 350, 1);
        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 100, false),
            1
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_withdraw_with_auditor(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer,
        bob: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset, &aptos_fx, &fa, &alice, &bob, 500, 500,
            );

        let alice_addr = signer::address_of(&alice);
        let bob_addr = signer::address_of(&bob);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();
        let (auditor_dk, auditor_ek) = generate_twisted_elgamal_keypair();

        confidential_asset::set_auditor_for_asset_type(
            &aptos_fx, token, option::some(auditor_ek.compressed_point_to_bytes()),
        );

        register(&alice, &alice_dk, alice_ek, token);

        confidential_asset::deposit(&alice, token, 200);
        confidential_asset::rollover_pending_balance(&alice, token);

        withdraw(&alice, &alice_dk, token, bob_addr, 50, 150);

        assert!(primary_fungible_store::balance(bob_addr, token) == 550, 1);
        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 150, false),
            1
        );
        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &auditor_dk, 150, true),
            1
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_transfer_test(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer,
        bob: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset,
                &aptos_fx,
                &fa,
                &alice,
                &bob,
                500,
                500
            );

        let alice_addr = signer::address_of(&alice);
        let bob_addr = signer::address_of(&bob);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();
        let (bob_dk, bob_ek) = generate_twisted_elgamal_keypair();

        register(&alice, &alice_dk, alice_ek, token);
        register(&bob, &bob_dk, bob_ek, token);

        confidential_asset::deposit(&alice, token, 200);
        confidential_asset::rollover_pending_balance(&alice, token);

        transfer(&alice, &alice_dk, token, bob_addr, 100, 100);

        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 100, false),
            1
        );
        assert!(
            confidential_asset::check_pending_balance_decrypts_to(bob_addr, token, &bob_dk, 100),
            1
        );

        transfer(&alice, &alice_dk, token, bob_addr, 100, 0);

        assert!(
            confidential_asset::check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 0, false),
            1
        );
        assert!(
            confidential_asset::check_pending_balance_decrypts_to(bob_addr, token, &bob_dk, 200),
            1
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_transfer_no_eff_1_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, false, 1,
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_transfer_no_eff_2_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, false, 2,
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_transfer_no_eff_3_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, false, 3,
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_transfer_eff_0_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, true, 0,
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_transfer_eff_1_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, true, 1,
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_transfer_eff_2_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, true, 2,
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_transfer_eff_3_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, true, 3,
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    #[expected_failure(abort_code = 65542, location = aptos_experimental::sigma_protocol_transfer)]
    fun fail_audit_transfer_if_wrong_auditor_count(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer,
        bob: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset,
                &aptos_fx,
                &fa,
                &alice,
                &bob,
                500,
                500
            );

        let bob_addr = signer::address_of(&bob);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();
        let (bob_dk, bob_ek) = generate_twisted_elgamal_keypair();
        let (_, auditor1_ek) = generate_twisted_elgamal_keypair();

        // No auditor set initially
        register(&alice, &alice_dk, alice_ek, token);
        register(&bob, &bob_dk, bob_ek, token);

        confidential_asset::deposit(&alice, token, 200);
        confidential_asset::rollover_pending_balance(&alice, token);

        // Prove without an effective auditor (none set on chain)
        let proof = confidential_asset::prove_transfer(
            signer::address_of(&alice),
            bob_addr,
            token,
            &alice_dk,
            100,
            100,
            &vector[], // no voluntary auditors
        );

        // Now set a global auditor AFTER proving
        confidential_asset::set_auditor_for_asset_type(
            &aptos_fx,
            token,
            std::option::some(auditor1_ek.compressed_point_to_bytes())
        );

        // Proof has amount_R_eff_aud = [] but chain now has effective auditor
        // → E_AUDITOR_COUNT_MISMATCH
        confidential_asset::confidential_transfer(
            &alice,
            token,
            bob_addr,
            proof
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1,
            bob = @0xb0
        )
    ]
    fun success_rotate(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer,
        bob: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset,
                &aptos_fx,
                &fa,
                &alice,
                &bob,
                500,
                500
            );

        let alice_addr = signer::address_of(&alice);
        let bob_addr = signer::address_of(&bob);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();

        register(&alice, &alice_dk, alice_ek, token);

        confidential_asset::deposit(&alice, token, 200);
        confidential_asset::rollover_pending_balance(&alice, token);

        withdraw(&alice, &alice_dk, token, bob_addr, 50, 150);

        // Must pause incoming transfers before key rotation (pending balance is already zero)
        confidential_asset::set_incoming_transfers_paused(&alice, token, true);

        let (new_alice_dk, new_alice_ek) = generate_twisted_elgamal_keypair();

        rotate(
            &alice,
            &alice_dk,
            token,
            &new_alice_dk,
        );

        assert!(confidential_asset::get_encryption_key(alice_addr, token) == new_alice_ek, 1);
        assert!(
            confidential_asset::check_available_balance_decrypts_to(
                alice_addr, token, &new_alice_dk, 150, false
            ),
            1
        );
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1
        )
    ]
    #[expected_failure(abort_code = 65545, location = confidential_asset)]
    fun fail_register_if_token_disallowed(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset,
                &aptos_fx,
                &fa,
                &alice,
                &alice,
                500,
                500
            );

        confidential_asset::set_allow_listing(&aptos_fx, true);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();

        register(&alice, &alice_dk, alice_ek, token);
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            fa = @0xfa,
            alice = @0xa1
        )
    ]
    fun success_register_if_token_allowed(
        confidential_asset: signer,
        aptos_fx: signer,
        fa: signer,
        alice: signer
    ) {
        let token =
            set_up_for_confidential_asset_test(
                &confidential_asset,
                &aptos_fx,
                &fa,
                &alice,
                &alice,
                500,
                500
            );

        confidential_asset::set_allow_listing(&aptos_fx, true);
        confidential_asset::set_confidentiality_for_asset_type(&aptos_fx, token, true);

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();

        register(&alice, &alice_dk, alice_ek, token);
    }

    #[
        test(
            confidential_asset = @aptos_experimental,
            aptos_fx = @aptos_framework,
            alice = @0xa1
        )
    ]
    fun fail_deposit_with_coins_if_insufficient_amount(
        confidential_asset: signer, aptos_fx: signer, alice: signer
    ) {
        chain_id::initialize_for_test(&aptos_fx, 4);
        confidential_asset::init_module_for_testing(&confidential_asset);
        coin::create_coin_conversion_map(&aptos_fx);

        let alice_addr = signer::address_of(&alice);

        let (burn_cap, freeze_cap, mint_cap) =
            coin::initialize<MockCoin>(
                &confidential_asset,
                utf8(b"MockCoin"),
                utf8(b"MC"),
                0,
                false
            );

        let coin_amount = coin::mint(100, &mint_cap);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);
        coin::destroy_mint_cap(mint_cap);

        account::create_account_if_does_not_exist(alice_addr);
        coin::register<MockCoin>(&alice);
        coin::deposit(alice_addr, coin_amount);

        coin::create_pairing<MockCoin>(&aptos_fx);

        let token = coin::paired_metadata<MockCoin>().extract();

        let (alice_dk, alice_ek) = generate_twisted_elgamal_keypair();

        register(&alice, &alice_dk, alice_ek, token);
        confidential_asset::deposit(&alice, token, 100);
    }
}
