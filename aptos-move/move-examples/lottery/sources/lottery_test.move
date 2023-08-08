module lottery::lottery_test {
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use aptos_framework::coin::{MintCapability, BurnCapability};
    #[test_only]
    use aptos_std::crypto_algebra::enable_cryptography_algebra_natives;
    #[test_only]
    use aptos_std::debug;
    #[test_only]
    use aptos_std::debug::print;
    #[test_only]
    use std::signer;
    #[test_only]
    use std::string;
    #[test_only]
    use std::string::utf8;
    #[test_only]
    use std::vector;
    #[test_only]
    use aptos_std::randomness;
    #[test_only]
    use lottery::lottery_common;
    #[test_only]
    use lottery::lottery_insecure;
    #[test_only]
    use lottery::lottery_secure;

    #[test_only]
    fun give_coins_for_one_ticket(mint_cap: &MintCapability<AptosCoin>, to: &signer) {
        let to_addr = signer::address_of(to);
        if (!account::exists_at(to_addr)) {
            account::create_account_for_test(to_addr);
        };
        coin::register<AptosCoin>(to);

        let coins = coin::mint(lottery_common::get_ticket_price(), mint_cap);
        coin::deposit(to_addr, coins);
    }

    #[test(
        deployer = @lottery,
        admin = @admin_address,
        fx = @aptos_framework,
        u1 = @0xA001, u2 = @0xA002, u3 = @0xA003, u4 = @0xA004
    )]
    fun test_lottery(
        deployer: signer,
        admin: &signer,
        fx: &signer,
        u1: &signer, u2: &signer, u3: &signer, u4: &signer,

    ) {
        enable_cryptography_algebra_natives(fx);
        randomness::initialize_for_testing(fx);

        // Deploy the lottery smart contracts
        account::create_account_for_test(signer::address_of(&deployer));
        lottery_insecure::init_module_for_testing(&deployer);
        lottery_secure::init_module_for_testing(&deployer);

        // Needed to mint coins out of thin air for testing
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(fx);

        debug::print(&utf8(b"test lottery_insecure begin"));
        randomness::set_seed(x"0000000000000000000000000000000000000000000000000000000000000000");
        test_lottery_internal(&mint_cap, &burn_cap, admin,
            u1, u2, u3, u4,
            1, 2, 3, 4,
            //8819839, 2, 3, 4,
            |admin, u1, u2, u3, u4, g1, g2, g3, g4| {
            // Each user sends a TXN to buy their ticket
            lottery_insecure::buy_a_ticket(u1, g1);
            lottery_insecure::buy_a_ticket(u2, g2);
            lottery_insecure::buy_a_ticket(u3, g3);
            lottery_insecure::buy_a_ticket(u4, g4);

            // Send a TXN  to close the lottery and determine the winners
            lottery_insecure::randomly_pick_winner_internal(admin)
        });
        debug::print(&utf8(b"test lottery_insecure finish"));
        debug::print(&utf8(b""));

        debug::print(&utf8(b"test lottery_secure done"));
        randomness::set_seed(x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        test_lottery_internal(&mint_cap, &burn_cap, admin,
            u1, u2, u3, u4,
            //1, 2, 3, 4,
            1, 2, 3, 4616533,
            |admin, u1, u2, u3, u4, g1, g2, g3, g4| {
            // Each user sends a TXN to buy their ticket
            lottery_secure::buy_a_ticket(u1, g1);
            lottery_secure::buy_a_ticket(u2, g2);
            lottery_secure::buy_a_ticket(u3, g3);
            lottery_secure::buy_a_ticket(u4, g4);

            // Send a TXN  to close the lottery and commit to the winners
            let winning_number = lottery_secure::commit_to_random_winner_internal(admin);
            debug::print(&utf8(b"Winning number was:"));
            debug::print(&winning_number);
            // Send another TXN to pay the winners
            lottery_secure::pay_out_winners_internal()
        });
        debug::print(&utf8(b"test lottery_secure finish"));

        // Clean up
        coin::destroy_burn_cap<AptosCoin>(burn_cap);
        coin::destroy_mint_cap<AptosCoin>(mint_cap);
    }

    #[test_only]
    inline fun test_lottery_internal(
        mint_cap: &MintCapability<AptosCoin>,
        burn_cap: &BurnCapability<AptosCoin>,
        admin: &signer,
        u1: &signer, u2: &signer, u3: &signer, u4: &signer,
        g1: u64, g2: u64, g3: u64, g4: u64,
        test_lottery_fn: |&signer, &signer, &signer, &signer, &signer, u64, u64, u64, u64| vector<address>
    ) {
        // Create fake coins for users participating in lottery & initialize aptos_framework
        give_coins_for_one_ticket(mint_cap, u1);
        give_coins_for_one_ticket(mint_cap, u2);
        give_coins_for_one_ticket(mint_cap, u3);
        give_coins_for_one_ticket(mint_cap, u4);

        let players = vector[
            signer::address_of(u1),
            signer::address_of(u2),
            signer::address_of(u3),
            signer::address_of(u4)
        ];

        debug::print(&string::utf8(b"The players are: "));
        vector::for_each_ref(&players, |addr| {
            debug::print(addr);
        });

        let jackpot = 4 * lottery_common::get_ticket_price();
        debug::print(&string::utf8(b"The jackpot is: "));
        print(&jackpot);
        let winners = test_lottery_fn(
            admin,
            u1, u2, u3, u4,
            g1, g2, g3, g4,
            //1,1, 0, 0,
        );
        let num_winners = vector::length(&winners);

        if (num_winners > 0) {
            debug::print(&string::utf8(b"The winners are: "));
            vector::for_each_ref(&winners, |addr| {
                debug::print(addr);
            });
        } else {
            debug::print(&string::utf8(b"There were no winners!"));
        };

        // Assert the winners got all the money
        vector::for_each_ref(&players, |addr| {
            let balance = coin::balance<AptosCoin>(*addr);
            let (found, _) = vector::find(&winners, |winner| winner == addr);
            if (!found) {
                assert!(balance == 0, 0);
            } else {
                assert!(balance == jackpot / num_winners, 0);
                // For subsequent tests to not be affected
                coin::burn_from(*addr, balance, burn_cap);
            }
        });
    }
}
