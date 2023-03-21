#[test_only]
module drand::lottery_test {
    use drand::lottery;
    use aptos_framework::timestamp;
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    use std::option;
    use aptos_std::debug;
    use std::string;
    use aptos_framework::coin::MintCapability;
    use std::vector;

    fun give_coins(mint_cap: &MintCapability<AptosCoin>, to: &signer) {
        let to_addr = signer::address_of(to);
        if (!account::exists_at(to_addr)) {
            account::create_account_for_test(to_addr);
        };
        coin::register<AptosCoin>(to);

        let coins = coin::mint(lottery::get_ticket_price(), mint_cap);
        coin::deposit(to_addr, coins);
    }

    #[test(myself = @drand, fx = @aptos_framework, u1 = @0xA001, u2 = @0xA002, u3 = @0xA003, u4 = @0xA004)]
    fun test_lottery(
        myself: signer, fx: signer,
        u1: signer, u2: signer, u3: signer, u4: signer,
    ) {
        timestamp::set_time_has_started_for_testing(&fx);

        // Deploy the lottery smart contract
        lottery::init_module(&myself);

        // Needed to mint coins out of thin air for testing
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(&fx);

        // We simulate different runs of the lottery to demonstrate the uniformity of the outcomes
        let vec_signed_bytes = vector::empty<vector<u8>>();
        vector::push_back(&mut vec_signed_bytes, x"c0ffeedeadbeef1337acbd123456789000"); // u1 wins
        vector::push_back(&mut vec_signed_bytes, x"c0ffeedeadbeef1337acbd123456789001"); // u2 wins
        vector::push_back(&mut vec_signed_bytes, x"c0ffeedeadbeef1337acbd123456789002"); // u3 wins
        vector::push_back(&mut vec_signed_bytes, x"c0ffeedeadbeef1337acbd123456789003"); // u3 wins
        vector::push_back(&mut vec_signed_bytes, x"c0ffeedeadbeef1337acbd123456789004"); // u4 wins
        vector::push_back(&mut vec_signed_bytes, x"c0ffeedeadbeef1337acbd123456789005"); // u3 wins
        vector::push_back(&mut vec_signed_bytes, x"c0ffeedeadbeef1337acbd123456789006"); // u3 wins
        vector::push_back(&mut vec_signed_bytes, x"c0ffeedeadbeef1337acbd123456789007"); // u1 wins

        let lottery_start_time_secs = 1677685200; // the time that the 1st drand epoch started
        let lottery_duration = lottery::get_minimum_lottery_duration_in_secs();

        // We pop_back, so we reverse the vector to simulate pop_front
        vector::reverse(&mut vec_signed_bytes);

        while(!vector::is_empty(&vec_signed_bytes)) {
            let signed_bytes = vector::pop_back(&mut vec_signed_bytes);

            // Create fake coins for users participating in lottery & initialize aptos_framework
            give_coins(&mint_cap, &u1);
            give_coins(&mint_cap, &u2);
            give_coins(&mint_cap, &u3);
            give_coins(&mint_cap, &u4);

            // Simulates the lottery starting at the current blockchain time
            timestamp::update_global_time_for_test(lottery_start_time_secs * 1000 * 1000);

            test_lottery_with_randomness(
                &u1, &u2, &u3, &u4,
                lottery_start_time_secs, lottery_duration,
                signed_bytes
            );

            // Shift the next lottery's start time a little (otherwise, timestamp::update_global_time_for_test fails
            // when resetting the time back to the past).
            lottery_start_time_secs = lottery_start_time_secs + 2 * lottery_duration;
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
        drand_signed_bytes: vector<u8>,
    ) {
        //debug::print(&string::utf8(b"The lottery duration is: "));
        //debug::print(&lottery_duration);
        //debug::print(&string::utf8(b"The time before starting it is: "));
        //debug::print(&timestamp::now_seconds());

        let lottery_draw_at_time = lottery_start_time_secs + lottery_duration;

        //
        // Send a TXN to start the lottery
        //
        lottery::start_lottery(lottery_draw_at_time);

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
        let winner_addr = option::extract(&mut lottery::close_lottery(drand_signed_bytes));

        debug::print(&string::utf8(b"The winner is: "));
        debug::print(&winner_addr)
    }
}
