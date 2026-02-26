#[test_only]
module aptos_experimental::confidential_proof_tests {
    use std::signer;
    use aptos_framework::account;
    use aptos_experimental::confidential_asset;
    use aptos_experimental::confidential_asset_tests;
    use aptos_experimental::ristretto255_twisted_elgamal::generate_twisted_elgamal_keypair;

    #[test]
    fun success_withdraw() {
        let aptos_fx = account::create_signer_for_test(@aptos_framework);
        let ca_signer = account::create_signer_for_test(@aptos_experimental);
        let fa_signer = account::create_signer_for_test(@0xfa);
        let sender = account::create_signer_for_test(@0xa1);
        let recipient = account::create_signer_for_test(@0xb0);

        let token = confidential_asset_tests::set_up_for_confidential_asset_test(
            &ca_signer, &aptos_fx, &fa_signer, &sender, &recipient, 500, 0
        );

        let (dk, ek) = generate_twisted_elgamal_keypair();
        let reg_proof = confidential_asset::prove_registration(signer::address_of(&sender), token, &dk);
        confidential_asset::register(&sender, token, ek, reg_proof);

        confidential_asset::deposit(&sender, token, 150);
        confidential_asset::rollover_pending_balance(&sender, token);

        let amount: u64 = 50;
        let new_amount: u128 = 100;

        let sender_addr = signer::address_of(&sender);
        let proof = confidential_asset::prove_withdrawal(sender_addr, token, &dk, amount, new_amount);

        let sender_ek = confidential_asset::get_encryption_key(sender_addr, token);
        let current_balance = confidential_asset::get_available_balance(sender_addr, token);
        let auditor_ek = confidential_asset::get_effective_auditor(token);

        confidential_asset::assert_valid_withdrawal_proof(
            &sender, token, &sender_ek, amount, &current_balance, &auditor_ek, proof
        );
    }

    // TODO: Rewrite `fail_withdraw_if_wrong_amount` as an SDK test
    // TODO: Rewrite `fail_withdraw_if_wrong_current_balance` as an SDK test
    // TODO: Rewrite `fail_withdraw_if_negative_new_balance` as an SDK test
    // TODO: Rewrite `fail_withdraw_if_wrong_new_balance` as an SDK test

    #[test]
    fun success_transfer() {
        let aptos_fx = account::create_signer_for_test(@aptos_framework);
        let ca_signer = account::create_signer_for_test(@aptos_experimental);
        let fa_signer = account::create_signer_for_test(@0xfa);
        let sender = account::create_signer_for_test(@0xa1);
        let recipient = account::create_signer_for_test(@0xb0);

        let token = confidential_asset_tests::set_up_for_confidential_asset_test(
            &ca_signer, &aptos_fx, &fa_signer, &sender, &recipient, 500, 500
        );

        let (sender_dk, sender_ek) = generate_twisted_elgamal_keypair();
        let (_, _recipient_ek) = generate_twisted_elgamal_keypair();

        let sender_addr = signer::address_of(&sender);
        let recipient_addr = signer::address_of(&recipient);

        let reg_proof = confidential_asset::prove_registration(sender_addr, token, &sender_dk);
        confidential_asset::register(&sender, token, sender_ek, reg_proof);
        // Register recipient so prove_transfer can read their EK
        let (recipient_dk, recipient_ek2) = generate_twisted_elgamal_keypair();
        let reg_proof2 = confidential_asset::prove_registration(recipient_addr, token, &recipient_dk);
        confidential_asset::register(&recipient, token, recipient_ek2, reg_proof2);

        confidential_asset::deposit(&sender, token, 150);
        confidential_asset::rollover_pending_balance(&sender, token);

        let amount: u64 = 50;
        let new_amount: u128 = 100;

        let (_, auditor_ek) = generate_twisted_elgamal_keypair();
        let auditor_eks = vector[auditor_ek];

        let (proof, _test_auditor_amounts) =
            confidential_asset::prove_transfer(
                sender_addr, recipient_addr, token,
                &sender_dk, amount, new_amount,
                &auditor_eks,
            );

        let sender_ek_compressed = confidential_asset::get_encryption_key(sender_addr, token);
        let recipient_ek_compressed = confidential_asset::get_encryption_key(recipient_addr, token);
        let current_balance = confidential_asset::get_available_balance(sender_addr, token);

        confidential_asset::assert_valid_transfer_proof(
            &sender, recipient_addr, token,
            &sender_ek_compressed, &recipient_ek_compressed,
            &current_balance, &auditor_eks,
            false, // has_effective_auditor: no on-chain auditor
            1, // num_extra_auditors: the one auditor is user-chosen
            proof
        );
    }

    // TODO: Rewrite `fail_transfer_if_wrong_sender_ek` as an SDK test
    // TODO: Rewrite `fail_transfer_if_wrong_recipient_ek` as an SDK test
    // TODO: Rewrite `fail_transfer_if_wrong_current_balance` as an SDK test
    // TODO: Rewrite `fail_transfer_if_negative_new_balance` as an SDK test
    // TODO: Rewrite `fail_transfer_if_wrong_sender_amount` as an SDK test
    // TODO: Rewrite `fail_transfer_if_wrong_recipient_amount` as an SDK test
    // TODO: Rewrite `fail_transfer_if_wrong_auditor_eks` as an SDK test
    // TODO: Rewrite `fail_transfer_if_wrong_auditor_amounts` as an SDK test

    #[test]
    fun success_normalize() {
        let aptos_fx = account::create_signer_for_test(@aptos_framework);
        let ca_signer = account::create_signer_for_test(@aptos_experimental);
        let fa_signer = account::create_signer_for_test(@0xfa);
        let sender = account::create_signer_for_test(@0xa1);
        let recipient = account::create_signer_for_test(@0xb0);

        let max_chunk_value: u64 = 1 << 16 - 1;
        let token = confidential_asset_tests::set_up_for_confidential_asset_test(
            &ca_signer, &aptos_fx, &fa_signer, &sender, &recipient, 2 * max_chunk_value, 0
        );

        let (dk, ek) = generate_twisted_elgamal_keypair();
        let sender_addr = signer::address_of(&sender);

        let reg_proof = confidential_asset::prove_registration(sender_addr, token, &dk);
        confidential_asset::register(&sender, token, ek, reg_proof);

        // Deposit twice to create an un-normalized balance after rollover
        confidential_asset::deposit(&sender, token, max_chunk_value);
        confidential_asset::deposit(&sender, token, max_chunk_value);
        confidential_asset::rollover_pending_balance(&sender, token);

        let amount: u128 = (2 * max_chunk_value as u128);

        let proof = confidential_asset::prove_normalization(sender_addr, token, &dk, amount);

        let sender_ek = confidential_asset::get_encryption_key(sender_addr, token);
        let current_balance = confidential_asset::get_available_balance(sender_addr, token);
        let auditor_ek = confidential_asset::get_effective_auditor(token);

        confidential_asset::assert_valid_normalization_proof(
            &sender, token, &sender_ek, &current_balance, &auditor_ek, proof
        );
    }

    // TODO: Rewrite `fail_normalize_if_wrong_ek` as an SDK test
    // TODO: Rewrite `fail_normalize_if_wrong_current_balance` as an SDK test
    // TODO: Rewrite `fail_normalize_if_wrong_new_balance` as an SDK test
}
