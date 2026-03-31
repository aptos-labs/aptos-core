#[test_only]
module aptos_framework::confidential_asset_tests {
    use std::features;
    use std::option;
    use std::signer;
    use std::string::utf8;
    use aptos_std::ristretto255::{Scalar, CompressedRistretto, new_scalar_from_u64};
    use aptos_framework::account;
    use aptos_framework::chain_id;
    use aptos_framework::coin;
    use aptos_framework::fungible_asset::{Self, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;

    use aptos_std::ristretto255_bulletproofs::RangeProof;
    use aptos_framework::confidential_asset;
    use aptos_framework::confidential_balance::{Pending, Available, Balance, CompressedBalance,
        new_pending_from_p_and_r, split_available_into_chunks, split_pending_into_chunks,
    };
    use aptos_framework::confidential_crypto_test_utils::{Self as crypto_test_utils,
        decompress_points, generate_twisted_elgamal_keypair,
        generate_available_randomness, new_available_from_amount,
        generate_pending_randomness, prove_range,
    };
    use aptos_framework::sigma_protocol_key_rotation;
    use aptos_framework::sigma_protocol_proof;
    use aptos_framework::sigma_protocol_proof_tests;
    use aptos_framework::sigma_protocol_registration;
    use aptos_framework::sigma_protocol_transfer;
    use aptos_framework::sigma_protocol_utils::points_clone;
    use aptos_framework::sigma_protocol_withdraw;

    struct MockCoin {}

    /// Registers a user with a real sigma protocol proof.
    fun register(
        sender: &signer,
        dk: &Scalar,
        ek: CompressedRistretto,
        token: Object<Metadata>,
    ) {
        let proof = prove_registration(signer::address_of(sender), token, dk);
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
        let proof = prove_withdrawal(
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
        let proof = prove_transfer(
            signer::address_of(sender), to, token, sender_dk, amount, new_amount, &vector[],
        );

        confidential_asset::confidential_transfer(sender, token, to, proof, vector[]);
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
        let proof = prove_transfer(
            signer::address_of(sender), to, token, sender_dk, amount, new_amount, volun_auditor_eks,
        );

        let eff_aud_amount = get_amount_ciphertext_for_effective_auditor(&proof);
        let volun_aud_amounts = get_amount_ciphertexts_for_volun_auditors(&proof);

        confidential_asset::confidential_transfer(sender, token, to, proof, vector[]);

        (eff_aud_amount, volun_aud_amounts)
    }

    fun rotate(
        sender: &signer,
        sender_dk: &Scalar,
        token: Object<Metadata>,
        new_dk: &Scalar,
    ) {
        let proof =
            prove_key_rotation(
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
            confidential_asset::set_asset_specific_auditor(
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

        let proof = prove_transfer(
            alice_addr, bob_addr, token, &alice_dk, 100, 100, &volun_aud_eks,
        );

        if (has_eff_auditor) {
            let eff_aud_amount = get_amount_ciphertext_for_effective_auditor(&proof);
            assert!(crypto_test_utils::check_decrypts_to(&eff_aud_amount, eff_aud_amount.get_R(), option::borrow(&eff_aud_dk), 100), 1);
        };

        if (num_volun_auditors > 0) {
            let volun_aud_amounts = get_amount_ciphertexts_for_volun_auditors(&proof);
            let i = 0;
            while (i < num_volun_auditors) {
                assert!(crypto_test_utils::check_decrypts_to(&volun_aud_amounts[i], volun_aud_amounts[i].get_R(), &volun_aud_dks[i], 100), 1);
                i = i + 1;
            };
        };

        confidential_asset::confidential_transfer(alice, token, bob_addr, proof, vector[]);

        assert!(
            check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 100, false),
            1
        );
        assert!(
            check_pending_balance_decrypts_to(bob_addr, token, &bob_dk, 100),
            1
        );

        if (has_eff_auditor) {
            assert!(
                check_available_balance_decrypts_to(
                    alice_addr, token, option::borrow(&eff_aud_dk), 100, true,
                ),
                1
            );
        };
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
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
            check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, 0),
            1
        );
        assert!(
            check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 0, false),
            1
        );
        assert!(confidential_asset::get_encryption_key(alice_addr, token) == alice_ek, 1);
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
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
            check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, 100),
            1
        );
        assert!(
            check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 0, false),
            1
        );
        assert!(confidential_asset::is_normalized(alice_addr, token));

        confidential_asset::rollover_pending_balance(&alice, token);

        assert!(
            check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, 0),
            1
        );
        assert!(
            check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 100, false),
            1
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
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
            check_pending_balance_decrypts_to(alice_addr, token, &alice_dk, 100),
            1
        );
        assert!(
            check_pending_balance_decrypts_to(bob_addr, token, &bob_dk, 150),
            1
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
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
            check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 150, false),
            1
        );

        withdraw(&alice, &alice_dk, token, alice_addr, 50, 100);

        assert!(primary_fungible_store::balance(alice_addr, token) == 350, 1);
        assert!(
            check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 100, false),
            1
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
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

        confidential_asset::set_asset_specific_auditor(
            &aptos_fx, token, option::some(auditor_ek.compressed_point_to_bytes()),
        );

        register(&alice, &alice_dk, alice_ek, token);

        confidential_asset::deposit(&alice, token, 200);
        confidential_asset::rollover_pending_balance(&alice, token);

        withdraw(&alice, &alice_dk, token, bob_addr, 50, 150);

        assert!(primary_fungible_store::balance(bob_addr, token) == 550, 1);
        assert!(
            check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 150, false),
            1
        );
        assert!(
            check_available_balance_decrypts_to(alice_addr, token, &auditor_dk, 150, true),
            1
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
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
            check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 100, false),
            1
        );
        assert!(
            check_pending_balance_decrypts_to(bob_addr, token, &bob_dk, 100),
            1
        );

        transfer(&alice, &alice_dk, token, bob_addr, 100, 0);

        assert!(
            check_available_balance_decrypts_to(alice_addr, token, &alice_dk, 0, false),
            1
        );
        assert!(
            check_pending_balance_decrypts_to(bob_addr, token, &bob_dk, 200),
            1
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
    fun success_transfer_no_eff_1_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, false, 1,
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
    fun success_transfer_no_eff_2_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, false, 2,
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
    fun success_transfer_no_eff_3_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, false, 3,
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
    fun success_transfer_eff_0_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, true, 0,
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
    fun success_transfer_eff_1_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, true, 1,
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
    fun success_transfer_eff_2_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, true, 2,
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
    fun success_transfer_eff_3_volun(
        confidential_asset: signer, aptos_fx: signer, fa: signer, alice: signer, bob: signer,
    ) {
        test_confidential_transfer_impl(
            &confidential_asset, &aptos_fx, &fa, &alice, &bob, true, 3,
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
    #[expected_failure(abort_code = 65542, location = aptos_framework::sigma_protocol_transfer)]
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
        let proof = prove_transfer(
            signer::address_of(&alice),
            bob_addr,
            token,
            &alice_dk,
            100,
            100,
            &vector[], // no voluntary auditors
        );

        // Now set a global auditor AFTER proving
        confidential_asset::set_asset_specific_auditor(
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
            proof,
            vector[],
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1, bob = @0xb0)]
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
            check_available_balance_decrypts_to(
                alice_addr, token, &new_alice_dk, 150, false
            ),
            1
        );
    }

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1)]
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

    #[test(confidential_asset = @aptos_framework, aptos_fx = @aptos_framework, fa = @0xfa, alice = @0xa1)]
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
            confidential_asset = @aptos_framework,
            aptos_fx = @aptos_framework,
            alice = @0xa1
        )
    ]
    #[expected_failure]
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
        // Alice only has 100 coins but tries to deposit 200 — should abort.
        confidential_asset::deposit(&alice, token, 200);
    }

    // === Proof generation and balance checking helpers ===

    fun check_pending_balance_decrypts_to(
        user: address,
        asset_type: Object<Metadata>,
        user_dk: &Scalar,
        amount: u64
    ): bool {
        let pending_balance =
            confidential_asset::get_pending_balance(user, asset_type).decompress();

        crypto_test_utils::check_decrypts_to(&pending_balance, pending_balance.get_R(), user_dk, (amount as u128))
    }

    /// Checks that the available balance decrypts to `amount`.
    /// When `is_auditor_dk` is true, decrypts the auditor's ciphertext rather than the user's.
    fun check_available_balance_decrypts_to(
        user: address,
        asset_type: Object<Metadata>,
        dk: &Scalar,
        amount: u128,
        is_auditor_dk: bool,
    ): bool {
        let available_balance =
            confidential_asset::get_available_balance(user, asset_type).decompress();

        let decrypt_R = if (is_auditor_dk) { available_balance.get_R_aud() } else { available_balance.get_R() };
        crypto_test_utils::check_decrypts_to(&available_balance, decrypt_R, dk, amount)
    }

    fun get_amount_ciphertext_for_effective_auditor(proof: &confidential_asset::TransferProof): Balance<Pending> {
        let compressed_amount = confidential_asset::get_transfer_proof_compressed_amount(proof);
        let p = decompress_points(compressed_amount.get_compressed_P());
        let r_eff_aud = decompress_points(compressed_amount.get_compressed_R_eff_aud());
        new_pending_from_p_and_r(p, r_eff_aud)
    }

    fun get_amount_ciphertexts_for_volun_auditors(proof: &confidential_asset::TransferProof): vector<Balance<Pending>> {
        let compressed_amount = confidential_asset::get_transfer_proof_compressed_amount(proof);
        let p = decompress_points(compressed_amount.get_compressed_P());
        compressed_amount.get_compressed_R_volun_auds().map_ref(|r| {
            new_pending_from_p_and_r(points_clone(&p), decompress_points(r))
        })
    }

    fun prove_registration(
        sender_addr: address,
        asset_type: Object<Metadata>,
        dk: &Scalar,
    ): confidential_asset::RegistrationProof {
        let (stmt, witn) = sigma_protocol_registration::compute_statement_and_witness(dk);
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);
        let session = sigma_protocol_registration::new_session(&sender, asset_type);
        confidential_asset::new_registration_proof(session.prove(&stmt, &witn))
    }

    fun prove_withdrawal_internal(
        sender_addr: address,
        asset_type: Object<Metadata>,
        dk_sender: &Scalar,
        v: u64,
        new_amount: u128,
    ): (CompressedBalance<Available>, RangeProof, sigma_protocol_proof::Proof) {
        let ek = confidential_asset::get_encryption_key(sender_addr, asset_type);
        let compressed_ek_aud = confidential_asset::get_effective_auditor_ek(asset_type);
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);

        let new_balance_randomness = generate_available_randomness();
        let new_balance = new_available_from_amount(
            new_amount, &new_balance_randomness, &ek, &compressed_ek_aud
        );

        let new_r = new_balance_randomness.scalars();
        let new_a = split_available_into_chunks(new_amount);
        let zkrp_new_balance = prove_range(&new_a, new_r);

        let v = new_scalar_from_u64(v);
        let compressed_old_balance = confidential_asset::get_available_balance(sender_addr, asset_type);
        let compressed_new_balance = new_balance.compress();
        let (stmt, _) = sigma_protocol_withdraw::new_withdrawal_statement(
            ek, &compressed_old_balance, &compressed_new_balance, &compressed_ek_aud, v
        );
        let witn = sigma_protocol_proof_tests::new_withdrawal_witness(*dk_sender, new_a, *new_r);
        let session = sigma_protocol_withdraw::new_session(&sender, asset_type, compressed_ek_aud.is_some());
        (compressed_new_balance, zkrp_new_balance, session.prove(&stmt, &witn))
    }

    fun prove_withdrawal(
        sender_addr: address,
        asset_type: Object<Metadata>,
        sender_dk: &Scalar,
        amount: u64,
        new_amount: u128,
    ): confidential_asset::WithdrawalProof {
        let (compressed_new_balance, zkrp_new_balance, sigma) =
            prove_withdrawal_internal(sender_addr, asset_type, sender_dk, amount, new_amount);
        confidential_asset::new_withdrawal_proof(compressed_new_balance, zkrp_new_balance, sigma)
    }

    fun prove_normalization(
        sender_addr: address,
        asset_type: Object<Metadata>,
        sender_dk: &Scalar,
        amount: u128,
    ): confidential_asset::WithdrawalProof {
        let (compressed_new_balance, zkrp_new_balance, sigma) =
            prove_withdrawal_internal(sender_addr, asset_type, sender_dk, 0, amount);
        confidential_asset::new_withdrawal_proof(compressed_new_balance, zkrp_new_balance, sigma)
    }

    fun prove_transfer(
        sender_addr: address,
        recipient_addr: address,
        asset_type: Object<Metadata>,
        sender_dk: &Scalar,
        amount_u64: u64,
        new_balance_u128: u128,
        compressed_ek_volun_auds: &vector<CompressedRistretto>,
    ): confidential_asset::TransferProof {
        let ek_sender = confidential_asset::get_encryption_key(sender_addr, asset_type);
        let ek_recipient = confidential_asset::get_encryption_key(recipient_addr, asset_type);
        let compressed_old_balance = confidential_asset::get_available_balance(sender_addr, asset_type);
        let compressed_ek_eff_aud = confidential_asset::get_effective_auditor_ek(asset_type);
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);
        let has_effective_auditor = compressed_ek_eff_aud.is_some();
        let num_volun_auditors = compressed_ek_volun_auds.length();

        let new_balance_randomness = generate_available_randomness();
        let amount_randomness = generate_pending_randomness();

        let (stmt, witn, new_balance, amount) =
            sigma_protocol_proof_tests::build_transfer_statement_and_witness(
                sender_dk, &ek_sender, &ek_recipient, &compressed_old_balance,
                &compressed_ek_eff_aud, compressed_ek_volun_auds,
                amount_u64, new_balance_u128, &new_balance_randomness, &amount_randomness,
            );

        let new_a = split_available_into_chunks(new_balance_u128);
        let v = split_pending_into_chunks((amount_u64 as u128));
        let zkrp_new_balance = prove_range(&new_a, new_balance_randomness.scalars());
        let zkrp_amount = prove_range(&v, amount_randomness.scalars());

        let session = sigma_protocol_transfer::new_session(
            &sender, recipient_addr, asset_type, has_effective_auditor, num_volun_auditors,
        );

        confidential_asset::new_transfer_proof(
            new_balance.compress(),
            amount.compress(),
            *compressed_ek_volun_auds,
            zkrp_new_balance, zkrp_amount, session.prove(&stmt, &witn)
        )
    }

    fun prove_key_rotation(
        owner_addr: address,
        asset_type: Object<Metadata>,
        sender_dk: &Scalar,
        new_dk: &Scalar,
    ): confidential_asset::KeyRotationProof {
        let owner = aptos_framework::account::create_signer_for_test(owner_addr);

        // Get old EK and available balance
        let compressed_old_ek = confidential_asset::get_encryption_key(owner_addr, asset_type);
        let available_balance = confidential_asset::get_available_balance(owner_addr, asset_type);

        // Build statement and witness using the helper
        let (stmt, witn, compressed_new_ek, compressed_new_R) =
            sigma_protocol_key_rotation::compute_statement_and_witness_from_keys_and_old_ctxt(
                sender_dk, new_dk,
                compressed_old_ek,
                available_balance.get_compressed_R(),
            );

        // Prove
        let session = sigma_protocol_key_rotation::new_session(&owner, asset_type);

        confidential_asset::new_key_rotation_proof(
            compressed_new_ek,
            compressed_new_R,
            session.prove(&stmt, &witn),
        )
    }
}
