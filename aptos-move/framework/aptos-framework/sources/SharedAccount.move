module AptosFramework::SharedAccount {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::Account;
    use AptosFramework::Coin;
    use AptosFramework::SimpleMap::{Self, SimpleMap};
    use AptosFramework::ResourceAccount;

    struct Shares has store {
        numerator: u64,
        denominator: u64,
    }

    // Resource representing a shared account
    struct SharedAccount has key {
        share_record: SimpleMap<address, Shares>,
    }

    const EINVALID_INIT_INPUT: u64 = 0;
    const EACCOUNT_DNE: u64 = 1;
    const EADDRESS_ALREADY_EXISTS: u64 = 2;
    const EINVALID_NUMERATOR_DENOMINATOR_COMBINATIONS: u64 = 3;
    const EINVALID_SIGNER: u64 = 4;
    const EINSUFFICIENT_BALANCE: u64 = 5;

    public(script) fun initialize_shared_account(source: &signer, seed: vector<u8>): (signer) {
        assert!(Account::exists_at(Signer::address_of(source)), Errors::invalid_argument(EACCOUNT_DNE));
        let resource_signer = ResourceAccount::create_resource_account(source, seed, Vector::empty());
        if (!exists<SharedAccount>(Signer::address_of(&resource_signer))) {
            move_to(
                &resource_signer,
                SharedAccount {
                    share_record: SimpleMap::create(),
                }
            )
        };
        resource_signer
    }

    public(script) fun initialize(source: &signer, seed: vector<u8>, addresses: vector<address>, numerators: vector<u64>, denominator: u64): (signer) acquires SharedAccount {
        assert!(Vector::length(&addresses) == Vector::length(&numerators), Errors::invalid_argument(EINVALID_INIT_INPUT));

        let resource_signer = initialize_shared_account(source, seed);

        let shared_account = borrow_global_mut<SharedAccount>(Signer::address_of(&resource_signer));

        let sum: u64 = 0;
        let i = 0;

        while (i < Vector::length(&numerators)) {
            let num = *Vector::borrow(&numerators, i);
            let addr = *Vector::borrow(&addresses, i);
            assert!(Account::exists_at(addr), Errors::invalid_argument(EACCOUNT_DNE));
            sum = sum + num;
            i = i + 1;

            SimpleMap::add(&mut shared_account.share_record, addr, Shares { numerator: num, denominator: denominator });
        };
        assert!(sum == denominator, Errors::invalid_argument(EINVALID_NUMERATOR_DENOMINATOR_COMBINATIONS));
        resource_signer
    }

    public(script) fun disperse<CoinType>(resource_signer: &signer, total_amount: u64) acquires SharedAccount {
        assert!(exists<SharedAccount>(Signer::address_of(resource_signer)), Errors::invalid_argument(EINVALID_SIGNER));
        assert!(Coin::balance<CoinType>(Signer::address_of(resource_signer)) >= total_amount, Errors::limit_exceeded(EINSUFFICIENT_BALANCE));
                
        let i = 0;
        let shared_account = borrow_global<SharedAccount>(Signer::address_of(resource_signer));

        while (i < SimpleMap::length(&shared_account.share_record)) {
            let (key, value) = SimpleMap::get_entry(&shared_account.share_record, i);
            let current_amount = value.numerator * total_amount / value.denominator;
            Coin::transfer<CoinType>(resource_signer, *key, current_amount);
            i = i + 1;
        };
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113)]
    public(script) fun test_initialize(user: signer, test_user1: signer, test_user2: signer) : (signer) acquires SharedAccount {
        let addresses = Vector::empty<address>();
        let numerators = Vector::empty<u64>();
        let denominator: u64 = 5;
        let seed = x"01";
        let user_addr = Signer::address_of(&user);
        let user_addr1 = Signer::address_of(&test_user1);
        let user_addr2 = Signer::address_of(&test_user2);
 
        Account::create_account(user_addr);
        Account::create_account(user_addr1);
        Account::create_account(user_addr2);

        Vector::push_back(&mut addresses, user_addr1);
        Vector::push_back(&mut addresses, user_addr2);

        Vector::push_back(&mut numerators, 2);
        Vector::push_back(&mut numerators, 3);

        initialize(&user, seed, addresses, numerators, denominator)
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113)]
    #[expected_failure]
    public(script) fun test_initialize_with_incorrect_numerator_denominator_combo(user: signer, test_user1: signer, test_user2: signer) acquires SharedAccount {
        let addresses = Vector::empty<address>();
        let numerators = Vector::empty<u64>();
        let denominator: u64 = 10;
        let seed = x"01";
        let user_addr = Signer::address_of(&user);
        let user_addr1 = Signer::address_of(&test_user1);
        let user_addr2 = Signer::address_of(&test_user2);
 
        Account::create_account(user_addr);
        Account::create_account(user_addr1);
        Account::create_account(user_addr2);

        Vector::push_back(&mut addresses, user_addr1);
        Vector::push_back(&mut addresses, user_addr2);

        Vector::push_back(&mut numerators, 2);
        Vector::push_back(&mut numerators, 3);

        initialize(&user, seed, addresses, numerators, denominator);
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113, core_resources = @CoreResources, core_framework = @AptosFramework)]
    public(script) fun test_disperse(user: signer, test_user1: signer, test_user2: signer, core_resources: signer, core_framework: signer) acquires SharedAccount {
        use AptosFramework::TestCoin::{Self, TestCoin};

        let user_addr1 = Signer::address_of(&test_user1);
        let user_addr2 = Signer::address_of(&test_user2);
        let resource_signer = test_initialize(user, test_user1, test_user2);
        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);

        Coin::register<TestCoin>(&resource_signer);
        TestCoin::mint(&core_framework, Signer::address_of(&resource_signer), 1000);        
        disperse<TestCoin>(&resource_signer, 1000);
        Coin::destroy_mint_cap<TestCoin>(mint_cap);
        Coin::destroy_burn_cap<TestCoin>(burn_cap);

        assert!(Coin::balance<TestCoin>(user_addr1) == 400, 0);
        assert!(Coin::balance<TestCoin>(user_addr2) == 600, 1);
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113)]
    #[expected_failure]
    public(script) fun test_disperse_insufficient_balance(user: signer, test_user1: signer, test_user2: signer) acquires SharedAccount {
        use AptosFramework::TestCoin::TestCoin;

        let resource_signer = test_initialize(user, test_user1, test_user2);
        disperse<TestCoin>(&resource_signer, 1000);
    }
}