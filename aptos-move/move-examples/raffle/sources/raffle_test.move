module raffle::raffle_test {
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use aptos_framework::coin::MintCapability;

    #[test_only]
    use aptos_std::debug;

    #[test_only]
    use std::signer;
    #[test_only]
    use std::string;
    #[test_only]
    use std::vector;

    #[test_only]
    use raffle::raffle;

    #[test_only]
    use aptos_std::crypto_algebra::enable_cryptography_algebra_natives;
    #[test_only]
    use aptos_framework::randomness;

    #[test_only]
    fun give_coins(mint_cap: &MintCapability<AptosCoin>, to: &signer) {
        let to_addr = signer::address_of(to);
        if (!account::exists_at(to_addr)) {
            account::create_account_for_test(to_addr);
        };
        coin::register<AptosCoin>(to);

        let coins = coin::mint(raffle::get_ticket_price(), mint_cap);
        coin::deposit(to_addr, coins);
    }

    #[test(
        deployer = @raffle,
        fx = @aptos_framework,
        u1 = @0xA001, u2 = @0xA002, u3 = @0xA003, u4 = @0xA004
    )]
    fun test_raffle(
        deployer: signer,
        fx: signer,
        u1: signer, u2: signer, u3: signer, u4: signer,
    ) {
        enable_cryptography_algebra_natives(&fx);
        randomness::initialize_for_testing(&fx);

        // Deploy the raffle smart contract
        account::create_account_for_test(signer::address_of(&deployer));
        raffle::init_module_for_testing(&deployer);

        // Needed to mint coins out of thin air for testing
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(&fx);

        // Create fake coins for users participating in raffle & initialize aptos_framework
        give_coins(&mint_cap, &u1);
        give_coins(&mint_cap, &u2);
        give_coins(&mint_cap, &u3);
        give_coins(&mint_cap, &u4);


        let winner = test_raffle_with_randomness(
            &u1, &u2, &u3, &u4,
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
                assert!(coin::balance<AptosCoin>(player) == raffle::get_ticket_price() * num_players, 1);
            } else {
                assert!(coin::balance<AptosCoin>(player) == 0, 1);
            };

            i = i + 1;
        };

        // Clean up
        coin::destroy_burn_cap<AptosCoin>(burn_cap);
        coin::destroy_mint_cap<AptosCoin>(mint_cap);
    }

    #[test_only]
    fun test_raffle_with_randomness(
        u1: &signer, u2: &signer, u3: &signer, u4: &signer,
    ): address {
        //
        // Each user sends a TXN to buy their ticket
        //
        raffle::buy_a_ticket(u1);
        raffle::buy_a_ticket(u2);
        raffle::buy_a_ticket(u3);
        raffle::buy_a_ticket(u4);

        //
        // Send a TXN  to close the raffle and determine the winner
        //
        let winner_addr = raffle::randomly_pick_winner_internal();

        debug::print(&string::utf8(b"The winner is: "));
        debug::print(&winner_addr);

        winner_addr
    }
}
