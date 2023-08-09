module lottery::lottery_test {
    use aptos_framework::timestamp;
    use aptos_framework::account;
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    use aptos_framework::coin;
    use aptos_framework::coin::MintCapability;

    use aptos_std::debug;

    use std::signer;
    use std::string;
    use std::vector;

    use lottery::lottery;

    #[test_only]
    use aptos_std::crypto_algebra::enable_cryptography_algebra_natives;
    #[test_only]
    use aptos_framework::resource_account;

    #[test_only]
    fun give_coins(mint_cap: &MintCapability<AptosCoin>, to: &signer) {
        let to_addr = signer::address_of(to);
        if (!account::exists_at(to_addr)) {
            account::create_account_for_test(to_addr);
        };
        coin::register<AptosCoin>(to);

        let coins = coin::mint(lottery::get_ticket_price(), mint_cap);
        coin::deposit(to_addr, coins);
    }

    // NOTE: The long `resource_account` address is what's obtained when calling
    // ```
    //   resource_account::create_resource_account(
    //      &developer_account,
    //      vector::empty<u8>(),
    //      vector::empty<u8>()
    //   );
    // ```
    // because @developer_address is set to `0xcafe` in the `Move.toml`.
    //
    // You can double check with:
    // ```
    //   debug::print(&create_resource_address(&signer::address_of(&developer_account), b""));
    // ```
    #[test(
        developer_account = @developer_address,
        resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5,
        fx = @aptos_framework,
        u1 = @0xA001, u2 = @0xA002, u3 = @0xA003, u4 = @0xA004
    )]
    fun test_lottery(
        developer_account: signer,
        resource_account: signer,
        fx: signer,
        u1: signer, u2: signer, u3: signer, u4: signer,
    ) {
        enable_cryptography_algebra_natives(&fx);
        timestamp::set_time_has_started_for_testing(&fx);

        // Deploy the lottery smart contract
        account::create_account_for_test(signer::address_of(&developer_account));
        resource_account::create_resource_account(&developer_account, vector::empty<u8>(), vector::empty<u8>());
        lottery::init_module(&resource_account);

        // Needed to mint coins out of thin air for testing
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(&fx);

        let lottery_start_time_secs = 1677685200;
        let lottery_duration = lottery::get_minimum_lottery_duration_in_secs();

        // Create fake coins for users participating in lottery & initialize aptos_framework
        give_coins(&mint_cap, &u1);
        give_coins(&mint_cap, &u2);
        give_coins(&mint_cap, &u3);
        give_coins(&mint_cap, &u4);

        // Simulates the lottery starting at the current blockchain time
        timestamp::update_global_time_for_test(lottery_start_time_secs * 1000 * 1000);

        let winner = test_lottery_with_randomness(
            &u1, &u2, &u3, &u4,
            lottery_start_time_secs, lottery_duration,
        );

        let players = vector[
            signer::address_of(&u1),
            signer::address_of(&u2),
            signer::address_of(&u3),
            signer::address_of(&u4)
        ];

        // Assert the winner got all the money
        let i = 0;
        let num_players = vector::length(&players);
        while (i < num_players) {
            let player = *vector::borrow(&players, i);

            if (player == winner) {
                assert!(coin::balance<AptosCoin>(player) == lottery::get_ticket_price() * num_players, 1);
            } else {
                assert!(coin::balance<AptosCoin>(player) == 0, 1);
            };

            i = i+1;
        };

        // Clean up
        coin::destroy_burn_cap<AptosCoin>(burn_cap);
        coin::destroy_mint_cap<AptosCoin>(mint_cap);
    }

    #[test_only]
    fun test_lottery_with_randomness(
        u1: &signer, u2: &signer, u3: &signer, u4: &signer,
        lottery_start_time_secs: u64,
        lottery_duration: u64,
    ): address {
        //debug::print(&string::utf8(b"The lottery duration is: "));
        //debug::print(&lottery_duration);
        //debug::print(&string::utf8(b"The time before starting it is: "));
        //debug::print(&timestamp::now_seconds());

        let lottery_draw_at_time = lottery_start_time_secs + lottery_duration;

        //
        // Send a TXN to start the lottery
        //
        lottery::start_lottery();

        //
        // Each user sends a TXN to buy their ticket
        //
        lottery::buy_a_ticket(u1);
        lottery::buy_a_ticket(u2);
        lottery::buy_a_ticket(u3);
        lottery::buy_a_ticket(u4);

        // Advance time far enough so the lottery can be closed
        timestamp::fast_forward_seconds(lottery_duration);
        assert!(timestamp::now_seconds() == lottery_draw_at_time, 1);
        //debug::print(&string::utf8(b"The time before closing is: "));
        //debug::print(&timestamp::now_seconds());

        //
        // Send a TXN with `drand_signed_bytes` to close the lottery and determine the winner
        //
        let winner_addr = lottery::decide_winners_internal();

        debug::print(&string::utf8(b"The winner is: "));
        debug::print(&winner_addr);

        winner_addr
    }
}
