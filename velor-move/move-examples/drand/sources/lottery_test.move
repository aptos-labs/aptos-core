module drand::lottery_test {
    #[test_only]
    use drand::lottery;
    #[test_only]
    use velor_framework::timestamp;
    #[test_only]
    use std::signer;
    #[test_only]
    use velor_framework::account;
    #[test_only]
    use velor_framework::coin;
    #[test_only]
    use velor_framework::velor_coin::{Self, VelorCoin};
    #[test_only]
    use velor_framework::coin::MintCapability;
    #[test_only]
    use std::vector;
    #[test_only]
    use std::string;
    #[test_only]
    use std::debug;
    #[test_only]
    use velor_std::crypto_algebra::enable_cryptography_algebra_natives;

    #[test_only]
    fun give_coins(mint_cap: &MintCapability<VelorCoin>, to: &signer) {
        let to_addr = signer::address_of(to);
        if (!account::exists_at(to_addr)) {
            account::create_account_for_test(to_addr);
        };
        coin::register<VelorCoin>(to);

        let coins = coin::mint(lottery::get_ticket_price(), mint_cap);
        coin::deposit(to_addr, coins);
    }

    #[test(myself = @drand, fx = @velor_framework, u1 = @0xA001, u2 = @0xA002, u3 = @0xA003, u4 = @0xA004)]
    fun test_lottery(
        myself: signer, fx: signer,
        u1: signer, u2: signer, u3: signer, u4: signer,
    ) {
        enable_cryptography_algebra_natives(&fx);
        timestamp::set_time_has_started_for_testing(&fx);

        // Needed to mint coins out of thin air for testing
        let (burn_cap, mint_cap) = velor_coin::initialize_for_test(&fx);

        // Deploy the lottery smart contract
        lottery::init_module_for_testing(&myself);

        // We simulate different runs of the lottery to demonstrate the uniformity of the outcomes
        let vec_signed_bytes = vector::empty<vector<u8>>();
        // curl https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/202
        vector::push_back(&mut vec_signed_bytes, x"a438d55a0a3aeff6c6b78ad40c2dfb55dae5154d86eeb8163138f2bf96294f90841e75ad952bf8101630da7bb527da21"); // u1 wins.
        // curl https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/602
        vector::push_back(&mut vec_signed_bytes, x"b0e64fd43f49f3cf20135e7133112c0ae461e6a7b2961ef474f716648a9ab5b67f606af2980944344de131ab970ccb5d"); // u1 wins.
        // curl https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/1002
        vector::push_back(&mut vec_signed_bytes, x"8a9b54d4790bcc1e0b8b3e452102bfc091d23ede4b488cb81580f37a52762a283ed8c8dd844f0a112fda3d768ec3f9a2"); // u4 wins.
        // curl https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/1402
        vector::push_back(&mut vec_signed_bytes, x"8eaca04732b0de0c2a385f0ccaab9504592fcae7ca621bef58302d4ef0bd2ce3dd9c90153688dedd47efdbeb4d9ecde5"); // u3 wins.

        let lottery_start_time_secs = 1677685200; // the time that the 1st drand epoch started
        let lottery_duration = lottery::get_minimum_lottery_duration_in_secs();

        // We pop_back, so we reverse the vector to simulate pop_front
        vector::reverse(&mut vec_signed_bytes);

        while(!vector::is_empty(&vec_signed_bytes)) {
            let signed_bytes = vector::pop_back(&mut vec_signed_bytes);

            // Create fake coins for users participating in lottery & initialize velor_framework
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
        coin::destroy_burn_cap<VelorCoin>(burn_cap);
        coin::destroy_mint_cap<VelorCoin>(mint_cap);
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
        lottery::close_lottery(drand_signed_bytes);
        let winner_addr = lottery::get_lottery_winner();
        debug::print(&string::utf8(b"The winner is: "));
        debug::print(&winner_addr)

    }
}
