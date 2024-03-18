/// Tests for veiled coins.
///
/// TODO: improve testing framework; currently very cumbersome to set up a veiled payment test
/// TODO: test that payments to self return successfully (ideally, they should do nothing)
module confidential_asset::veiled_coin_tests {
    #[test_only]
    use std::features;
    #[test_only]
    use std::signer;
    #[test_only]
    use std::string::utf8;

    #[test_only]
    use aptos_std::ristretto255_bulletproofs as bulletproofs;
    #[test_only]
    use aptos_std::debug::print;
    #[test_only]
    use aptos_std::ristretto255_elgamal as elgamal;
    #[test_only]
    use aptos_std::ristretto255;
    #[test_only]
    use aptos_std::ristretto255_pedersen as pedersen;

    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::coin;

    #[test_only]
    use confidential_asset::confidential_asset;
    #[test_only]
    use confidential_asset::helpers::generate_elgamal_keypair;
    #[test_only]
    use confidential_asset::sigma_protos::{serialize_withdrawal_subproof, prove_withdrawal};
    #[test_only]
    use confidential_asset::sigma_protos;
    #[test_only]
    use bond_coin::bond_coin;
    #[test_only]
    use aptos_framework::managed_coin;

    //
    // Test-only functions
    //

    #[test_only]
    /// Initializes the `veiled_coin` module and sets up a `sender` account with `sender_amount` + `recipient_amount`
    /// of `FakeCoin`'s. Then, sends `recipient_amount` of coins from `sender` to `recipient`.
    ///
    /// Can be called with `sender` set to be equal to `recipient`.
    fun set_up_for_veiled_coin_test(
        veiled_coin: &signer,
        aptos_fx: signer,
        sender: &signer,
        recipient: &signer,
        sender_amount: u32,
        recipient_amount: u32,
    ) {
        // Assumption is that framework address is different than recipient and sender addresses
        assert!(signer::address_of(&aptos_fx) != signer::address_of(sender), 1);
        assert!(signer::address_of(&aptos_fx) != signer::address_of(recipient), 2);

        // Initialize the `veiled_coin` module & enable the feature
        confidential_asset::init_module_for_testing(veiled_coin);
        println(b"Initialized module.");
        features::change_feature_flags_for_testing(&aptos_fx, vector[features::get_bulletproofs_feature()], vector[]);
        println(b"Enabled feature flags.");

        // Set up an account for the framework address
        account::create_account_for_test(signer::address_of(&aptos_fx)); // needed in `coin::create_fake_money`
        account::create_account_for_test(signer::address_of(sender)); // needed in `coin::transfer`
        if (signer::address_of(recipient) != signer::address_of(sender)) {
            account::create_account_for_test(signer::address_of(recipient)); // needed in `coin::transfer`
        };
        println(b"Created accounts for test.");

        // Create `amount` of `FakeCoin` coins at the Aptos 0x1 address (must do) and register a `FakeCoin` coin
        // store for the `sender`.
        coin::create_fake_money(
            &aptos_fx,
            sender,
            confidential_asset::cast_u32_to_u64_amount(sender_amount + recipient_amount));
        println(b"Created fake money inside @aptos_framework");

        // Transfer some coins from the framework to the sender
        coin::transfer<coin::FakeMoney>(
            &aptos_fx,
            signer::address_of(sender),
            confidential_asset::cast_u32_to_u64_amount(sender_amount));
        println(b"Transferred some fake money to the sender.");

        // Transfer some coins from the sender to the recipient
        coin::register<coin::FakeMoney>(recipient);
        coin::transfer<coin::FakeMoney>(
            &aptos_fx,
            signer::address_of(recipient),
            confidential_asset::cast_u32_to_u64_amount(recipient_amount));
        println(b"Transferred some fake money to the recipient.");

        println(b"Sender balance (as u64):");
        print(&coin::balance<coin::FakeMoney>(signer::address_of(sender)));
        println(b"Sender balance (as u32):");
        print(&confidential_asset::clamp_u64_to_u32_amount(coin::balance<coin::FakeMoney>(signer::address_of(sender))));
        if (signer::address_of(recipient) != signer::address_of(sender)) {
            println(b"Recipient balance (as u64):");
            print(&coin::balance<coin::FakeMoney>(signer::address_of(recipient)));
            println(b"Sender balance (as u32):");
            print(&confidential_asset::clamp_u64_to_u32_amount(coin::balance<coin::FakeMoney>(signer::address_of(recipient))));
        } else {
            println(b"(Recipient equals sender)");
        };
    }

    #[test_only]
    /// Prints a string on its own line.
    public fun println(str: vector<u8>) {
        print(&utf8(str));
    }

    //
    // Tests
    //

    #[test(veiled_coin = @veiled_coin, aptos_fx = @aptos_framework, sender = @0xc0ffee, recipient = @0x1337)]
    fun veil_test(
        veiled_coin: signer,
        aptos_fx: signer,
        sender: signer,
        recipient: signer
    ) {
        println(b"Starting veil_test()...");
        println(b"@veiled_coin:");
        print(&@veiled_coin);
        println(b"@aptos_framework:");
        print(&@aptos_framework);

        // Split 500 and 500 between `sender` and `recipient`
        set_up_for_veiled_coin_test(
            &veiled_coin, aptos_fx, &sender, &recipient, 500u32, 500u32);

        // Register a veiled balance at the `recipient`'s account
        let (recipient_sk, recipient_pk) = generate_elgamal_keypair();
        confidential_asset::register<coin::FakeMoney>(&recipient, elgamal::pubkey_to_bytes(&recipient_pk));
        println(b"Registered recipient's veiled coin balance");

        // Veil 150 normal coins from the `sender`'s normal coin account to the `recipient`'s veiled coin account
        confidential_asset::veil_to<coin::FakeMoney>(&sender, signer::address_of(&recipient), 150u32);
        println(b"Sender veiled some coins over to the recipient");

        // Check the transfer occurred correctly: sender has 350 public coins, recipient has 150 (not-yet-)veiled coins
        assert!(coin::balance<coin::FakeMoney>(signer::address_of(&sender)) == confidential_asset::cast_u32_to_u64_amount(350u32), 1);
        assert!(confidential_asset::verify_opened_balance<coin::FakeMoney>(
            signer::address_of(&recipient), 150u32, &ristretto255::scalar_zero(), &recipient_pk), 1);

        // Register a veiled balance at the `sender`'s account
        let (_, sender_pk) = generate_elgamal_keypair();
        confidential_asset::register<coin::FakeMoney>(&sender, elgamal::pubkey_to_bytes(&sender_pk));

        // The `recipient` wants to unveil 50 coins (to the `sender`), so build a range proof for that.
        // (Note: Technically, because the balance is not yet actually-veiled, the range proof could be avoided here in
        //  a smarter design.)
        let recipient_new_balance = ristretto255::new_scalar_from_u32(100u32);
        let recipient_curr_balance = ristretto255::new_scalar_from_u32(150u32);
        let recipient_amount_unveiled = ristretto255::new_scalar_from_u32(50u32);
        let (new_balance_range_proof, _) = bulletproofs::prove_range_pedersen(
            &recipient_new_balance, &ristretto255::scalar_zero(),
            confidential_asset::get_max_bits_in_veiled_coin_value(), confidential_asset::get_veiled_coin_bulletproofs_dst());
        let new_balance_range_proof_bytes = bulletproofs::range_proof_to_bytes(&new_balance_range_proof);

        let curr_balance_ct = elgamal::new_ciphertext_with_basepoint(&recipient_curr_balance, &ristretto255::scalar_zero(), &recipient_pk);
        let new_balance_comm = pedersen::new_commitment_for_bulletproof(&recipient_new_balance, &ristretto255::scalar_zero());
        let new_balance_comm_bytes = pedersen::commitment_to_bytes(&new_balance_comm);

        // Compute a sigma proof which shows that the recipient's new balance ciphertext and commitment both encode
        // the same value. The commitment is necessary to ensure the value is binding
        let sigma_proof = prove_withdrawal(
            &recipient_sk,
            &recipient_pk,
            &curr_balance_ct,
            &new_balance_comm,
            &recipient_new_balance,
            &recipient_amount_unveiled,
            &ristretto255::scalar_zero());
        let sigma_proof_bytes = serialize_withdrawal_subproof(&sigma_proof);

        // Transfer `50` veiled coins from the `recipient` to the `sender`'s public balance
        confidential_asset::unveil_to<coin::FakeMoney>(
            &recipient, signer::address_of(&sender), 50u32, new_balance_comm_bytes, new_balance_range_proof_bytes, sigma_proof_bytes);

        // Check that the sender now has 350 + 50 = 400 public coins
        let sender_public_balance = coin::balance<coin::FakeMoney>(signer::address_of(&sender));
        assert!(sender_public_balance == confidential_asset::cast_u32_to_u64_amount(400u32), 1);
        // Check that the recipient now has 100 veiled coins
        assert!(confidential_asset::verify_opened_balance<coin::FakeMoney>(
            signer::address_of(&recipient), 100u32, &ristretto255::scalar_zero(), &recipient_pk), 1);
    }

    #[test(veiled_coin = @veiled_coin, aptos_fx = @aptos_framework, sender = @0x1337)]
    fun unveil_test(
        veiled_coin: signer,
        aptos_fx: signer,
        sender: signer,
    ) {
        println(b"Starting unveil_test()...");
        println(b"@veiled_coin:");
        print(&@veiled_coin);
        println(b"@aptos_framework:");
        print(&@aptos_framework);

        // Create a `sender` account with 500 `FakeCoin`'s
        set_up_for_veiled_coin_test(
            &veiled_coin, aptos_fx, &sender, &sender, 500, 0);

        // Register a veiled balance for the `sender`
        let (sender_sk, sender_pk) = generate_elgamal_keypair();
        confidential_asset::register<coin::FakeMoney>(&sender, elgamal::pubkey_to_bytes(&sender_pk));
        println(b"Registered the sender's veiled balance");

        // Veil 150 out of the `sender`'s 500 coins.
        //
        // Note: Sender initializes his veiled balance to 150 veiled coins, which is why we don't need its SK to decrypt
        // it in order to transact.
        confidential_asset::veil<coin::FakeMoney>(&sender, 150);
        println(b"Veiled 150 coins to the `sender`");

        println(b"Total veiled coins:");
        print(&confidential_asset::total_veiled_coins<coin::FakeMoney>());

        assert!(confidential_asset::total_veiled_coins<coin::FakeMoney>() == confidential_asset::cast_u32_to_u64_amount(150), 1);

        // The `unveil` function uses randomness zero for the ElGamal encryption of the amount
        let sender_new_balance = ristretto255::new_scalar_from_u32(100);
        let sender_curr_balance = ristretto255::new_scalar_from_u32(150);
        let sender_amount_unveiled = ristretto255::new_scalar_from_u32(50);
        let zero_randomness = ristretto255::scalar_zero();

        let (new_balance_range_proof, _) = bulletproofs::prove_range_pedersen(
            &sender_new_balance,
            &zero_randomness,
            confidential_asset::get_max_bits_in_veiled_coin_value(), confidential_asset::get_veiled_coin_bulletproofs_dst());

        let curr_balance_ct = elgamal::new_ciphertext_with_basepoint(&sender_curr_balance, &zero_randomness, &sender_pk);
        let new_balance_comm = pedersen::new_commitment_for_bulletproof(&sender_new_balance, &zero_randomness);
        let new_balance_comm_bytes = pedersen::commitment_to_bytes(&new_balance_comm);

        let sigma_proof = sigma_protos::prove_withdrawal(
            &sender_sk,
            &sender_pk,
            &curr_balance_ct,
            &new_balance_comm,
            &sender_new_balance,
            &sender_amount_unveiled,
            &zero_randomness);
        let sigma_proof_bytes = serialize_withdrawal_subproof(&sigma_proof);

        println(b"about to unveil");
        // Move 50 veiled coins into the public balance of the sender
        confidential_asset::unveil<coin::FakeMoney>(
            &sender,
            50,
            new_balance_comm_bytes,
            bulletproofs::range_proof_to_bytes(&new_balance_range_proof),
            sigma_proof_bytes);

        println(b"Remaining veiled coins, after `unveil` call:");
        print(&confidential_asset::total_veiled_coins<coin::FakeMoney>());

        assert!(confidential_asset::total_veiled_coins<coin::FakeMoney>() == confidential_asset::cast_u32_to_u64_amount(100), 1);

        assert!(confidential_asset::verify_opened_balance<coin::FakeMoney>(
            signer::address_of(&sender), 100, &zero_randomness, &sender_pk), 2);

        let remaining_public_balance = coin::balance<coin::FakeMoney>(signer::address_of(&sender));
        assert!(remaining_public_balance == confidential_asset::cast_u32_to_u64_amount(400), 3);
    }

    #[test(veiled_coin = @veiled_coin, aptos_fx = @aptos_framework, sender = @0xe7194662d2006cf307eafbb00387137dc95dfdd228ae0c8158cf3dfabb9972a0, recipient = @0x9ac6e327c67207e641be69940ab4abfe7a2920c969185ae2b8515e4fa45dce01)]
    fun basic_viability_test(
        veiled_coin: signer,
        aptos_fx: signer,
        sender: signer,
        recipient: signer
    ) {
        set_up_for_veiled_coin_test(&veiled_coin, aptos_fx, &sender, &recipient, 500, 500);
         aptos_framework::managed_coin::initialize<bond_coin::BondCoin>(
            &recipient,
            b"BOND Coin",
            b"BOND",
            8,
            false,
        );
        managed_coin::register<bond_coin::BondCoin>(&sender);
        managed_coin::register<bond_coin::BondCoin>(&recipient);

        managed_coin::mint<bond_coin::BondCoin>(&recipient, @0x9ac6e327c67207e641be69940ab4abfe7a2920c969185ae2b8515e4fa45dce01, 600000000000);
        managed_coin::mint<bond_coin::BondCoin>(&recipient, @0xe7194662d2006cf307eafbb00387137dc95dfdd228ae0c8158cf3dfabb9972a0, 600000000000);

        // Creates a balance of `b = 150` veiled coins at sender (requires registering a veiled coin store at 'sender')
        let sender_sk = std::option::extract(&mut ristretto255::new_scalar_from_bytes(x"49cfc15340eeb185e6c91b0d0e452e2c4e4711c413238af4a5909ad59883580c"));
        let sender_pk = elgamal::pubkey_from_secret_key(&sender_sk);
        print(&sender_pk);
        print(&sender_sk);
        confidential_asset::register<bond_coin::BondCoin>(&sender, elgamal::pubkey_to_bytes(&sender_pk));
        confidential_asset::veil<bond_coin::BondCoin>(&sender, 1000);
        println(b"Veiled 150 coins to the `sender`");
        // TODO: This throws an invariant violation (INTERNAL_TYPE_ERROR (code 2009))
        //print(&sender);

        // Make sure we are correctly keeping track of the normal coins veiled in this module
        let total_veiled_coins = confidential_asset::cast_u32_to_u64_amount(1000);
        assert!(confidential_asset::total_veiled_coins<bond_coin::BondCoin>() == total_veiled_coins, 1);

        // Transfer `v = 50` of these veiled coins to the recipient
        let amount_val = ristretto255::new_scalar_from_u32(500);
        let amount_rand = ristretto255::new_scalar_from_u32(0);

        // The commitment to the sender's new balance can use fresh randomness as we don't use the
        // new balance amount in a ciphertext
        let new_balance_rand = ristretto255::new_scalar_from_u32(0);
        let curr_balance_val = ristretto255::new_scalar_from_u32(1000);

        // The sender's new balance is 150 - 50 = 100
        let new_balance_val = ristretto255::new_scalar_from_u32(500);

        // No veiled transfers have been done yet so the sender's balance randomness is zero
        let curr_balance_ct = elgamal::new_ciphertext_with_basepoint(&curr_balance_val, &ristretto255::scalar_zero(), &sender_pk);
        let (new_balance_range_proof, _) = bulletproofs::prove_range_pedersen(
            &new_balance_val, &new_balance_rand,
            confidential_asset::get_max_bits_in_veiled_coin_value(), confidential_asset::get_veiled_coin_bulletproofs_dst());
        println(b"Computed range proof over the `sender`'s new balance");

        // Compute a range proof over the commitment to `v` and encrypt it under the `sender`'s PK
        let withdraw_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &sender_pk);
        let (amount_val_range_proof, _) = bulletproofs::prove_range_pedersen(
            &amount_val, &amount_rand,
            confidential_asset::get_max_bits_in_veiled_coin_value(), confidential_asset::get_veiled_coin_bulletproofs_dst());
        println(b"Computed range proof over the transferred amount");

        // Register the `recipient` for receiving veiled coins
        let recipient_pk = sender_pk;
        confidential_asset::register<bond_coin::BondCoin>(&recipient, elgamal::pubkey_to_bytes(&recipient_pk));
        println(b"Registered the `recipient` to receive veiled coins");
        // TODO: This throws an invariant violation (INTERNAL_TYPE_ERROR (code 2009))
        //print(&recipient);

        // Encrypt the transfered amount `v` under the `recipient`'s PK
        let deposit_ct = elgamal::new_ciphertext_with_basepoint(
            &amount_val, &amount_rand, &recipient_pk);

        let amount_comm = pedersen::new_commitment_for_bulletproof(&amount_val, &amount_rand);
        let new_balance_comm = pedersen::new_commitment_for_bulletproof(&new_balance_val, &new_balance_rand);
        println(b"Computed commitments to the amount to transfer and the sender's updated balance");

        // Prove that the two encryptions of `v` are to the same value
        let sigma_proof = sigma_protos::prove_transfer(
            &sender_pk,
            &sender_sk,
            &recipient_pk,
            &withdraw_ct,
            &deposit_ct,
            &amount_comm,
            &curr_balance_ct,
            &new_balance_comm,
            &amount_rand,
            &amount_val,
            &new_balance_rand,
            &new_balance_val);
        let sigma_proof_bytes = sigma_protos::serialize_transfer_subproof(&sigma_proof);
        println(b"Created sigma protocol proof");

        // Sanity check veiled balances
        assert!(confidential_asset::verify_opened_balance<bond_coin::BondCoin>(signer::address_of(&sender), 1000, &ristretto255::scalar_zero(), &sender_pk), 1);
        assert!(confidential_asset::verify_opened_balance<bond_coin::BondCoin>(signer::address_of(&recipient), 0, &ristretto255::scalar_zero(), &recipient_pk), 1);

        println(b"param1");
        print(&elgamal::ciphertext_to_bytes(&withdraw_ct));
        println(b"param2");
        print(&elgamal::ciphertext_to_bytes(&deposit_ct));
        print(&pedersen::commitment_to_bytes(&new_balance_comm));
        print(&pedersen::commitment_to_bytes(&amount_comm));
        print(&bulletproofs::range_proof_to_bytes(&new_balance_range_proof));
        print(&bulletproofs::range_proof_to_bytes(&amount_val_range_proof));
        print(&sigma_proof_bytes);

        // Execute the veiled transaction: no one will be able to tell 50 coins are being transferred.
        confidential_asset::fully_veiled_transfer<bond_coin::BondCoin>(
            &sender,
            signer::address_of(&recipient),
            elgamal::ciphertext_to_bytes(&withdraw_ct),
            elgamal::ciphertext_to_bytes(&deposit_ct),
            pedersen::commitment_to_bytes(&new_balance_comm),
            pedersen::commitment_to_bytes(&amount_comm),
            bulletproofs::range_proof_to_bytes(&new_balance_range_proof),
            bulletproofs::range_proof_to_bytes(&amount_val_range_proof),
            sigma_proof_bytes);
        println(b"Transferred veiled coins");

        // Compute the randomness of the sender's current balance
        let balance_rand = ristretto255::scalar_neg(&amount_rand);
        // Sanity check veiled balances
        assert!(confidential_asset::verify_opened_balance<bond_coin::BondCoin>(signer::address_of(&sender), 500, &balance_rand, &sender_pk), 1);
        assert!(confidential_asset::verify_opened_balance<bond_coin::BondCoin>(signer::address_of(&recipient), 500, &amount_rand, &recipient_pk), 1);

        assert!(confidential_asset::total_veiled_coins<bond_coin::BondCoin>() == total_veiled_coins, 1);

        // Drain the whole remaining balance of the sender
        let new_curr_balance_val = ristretto255::new_scalar_from_u32(500);
        let new_amount_val = ristretto255::new_scalar_from_u32(500);
        let new_new_balance_val = ristretto255::new_scalar_from_u32(0);
        let fresh_new_balance_rand = ristretto255::new_scalar_from_u32(0);

        // `unveil` doesn't change the randomness, so we reuse the `new_balance_rand` randomness from before
        let (new_new_balance_range_proof, _) = bulletproofs::prove_range_pedersen(
            &new_new_balance_val, &fresh_new_balance_rand,
            confidential_asset::get_max_bits_in_veiled_coin_value(), confidential_asset::get_veiled_coin_bulletproofs_dst());

        // Compute a pedersen commitment over the same values the range proof is done over to gurantee a binding commitment
        // to the sender's new balance. A sigma proof demonstrates the commitment and ciphertexts contain the same value and randomness
        let new_curr_balance_ct = elgamal::new_ciphertext_with_basepoint(&new_curr_balance_val, &balance_rand, &sender_pk);
        let new_new_balance_comm = pedersen::new_commitment_for_bulletproof(&new_new_balance_val, &fresh_new_balance_rand);
        let new_new_balance_comm_bytes = pedersen::commitment_to_bytes(&new_new_balance_comm);
        let sigma_proof = sigma_protos::prove_withdrawal(
            &sender_sk,
            &sender_pk,
            &new_curr_balance_ct,
            &new_new_balance_comm,
            &new_new_balance_val,
            &new_amount_val,
            &fresh_new_balance_rand);
        let sigma_proof_bytes = serialize_withdrawal_subproof(&sigma_proof);

        // Unveil all coins of the `sender`
        confidential_asset::unveil<bond_coin::BondCoin>(
            &sender, 500, new_new_balance_comm_bytes, bulletproofs::range_proof_to_bytes(&new_new_balance_range_proof), sigma_proof_bytes);
        println(b"Unveiled all 100 coins from the `sender`'s veiled balance");

        let total_veiled_coins = confidential_asset::cast_u32_to_u64_amount(500);
        assert!(confidential_asset::total_veiled_coins<bond_coin::BondCoin>() == total_veiled_coins, 1);

        // Sanity check veiled balances
        // assert!(confidential_asset::verify_opened_balance<bond_coin::BondCoin>(signer::address_of(&sender), 500, &balance_rand, &sender_pk), 1);
        // assert!(confidential_asset::verify_opened_balance<bond_coin::BondCoin>(signer::address_of(&recipient), 500, &amount_rand, &recipient_pk), 1);
    }
}
