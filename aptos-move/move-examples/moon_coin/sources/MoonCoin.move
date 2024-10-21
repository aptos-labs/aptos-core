//:!:>moon
module MoonCoin::moon_coin {

    // More detail about moon coin and test

    use std::signer::address_of;
    use std::string::utf8;
    use aptos_framework::coin::{initialize, destroy_freeze_cap,
        destroy_burn_cap, destroy_mint_cap, mint, register, deposit
    };
    #[test_only]
    use aptos_std::debug::print;

    #[test_only]
    use aptos_framework::account::create_account_for_test;
    #[test_only]
    use aptos_framework::coin::balance;


    struct MoonCoin {}

    fun init_module(sender: &signer) {
        let (burn_cap1,freeze_cap1,mint_cap1)=initialize<MoonCoin>(
           sender,
           utf8(b"Moon Coin"),
           utf8(b"MOON"),
           6,
           false,);
        let moon_coin = mint(1000000,&mint_cap1); //mint 1 moon coin
        register<MoonCoin>(sender);                             // register a store for moon coin
        deposit(address_of(sender),moon_coin);     //  deposite to account
        destroy_freeze_cap<MoonCoin>(freeze_cap1);
        destroy_burn_cap(burn_cap1);
        destroy_mint_cap(mint_cap1);
    }

    #[test(caller=@moon_coin)]
    fun test_moon_coin(caller:&signer){
        create_account_for_test(address_of(caller));       // create account on test
        init_module(caller);
        print(&utf8(b"Moon coin balance"));
        print(&balance<MoonCoin>(address_of(caller)));
    }

}
//<:!:moon
