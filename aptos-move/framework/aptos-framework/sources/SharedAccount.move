module AptosFramework::SharedAccount {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::Account;
    use AptosFramework::Coin;
    use AptosFramework::SimpleMap::{Self, SimpleMap};

    // The numerator and the denominator of a fraction
    struct State has store {
        numerator: u64,
        denominator: u64,
    }

    // Resource representing a shared account
    struct SharedAccount has key {
        share_record: SimpleMap<address, State>,
        signer_capability: Account::SignerCapability,
    }

    const EINVALID_INPUT: u64 = 0;
    const EACCOUNT_NOT_FOUND: u64 = 1;
    const EADDRESS_ALREADY_EXISTS: u64 = 2;
    const EINVALID_NUMERATOR_DENOMINATOR_COMBINATIONS: u64 = 3;
    const EINVALID_SIGNER: u64 = 4;
    const EINSUFFICIENT_BALANCE: u64 = 5;

    fun find_greatest_common_divisor(denominator1: u64, denominator2: u64) : (u64) {
        if (denominator1 == 0) {
            denominator2
        }
        else {
            find_greatest_common_divisor(denominator2 % denominator1, denominator1)
        }
    }

    fun find_least_common_denominator(denominator1: u64, denominator2: u64) : (u64) {
        assert!(denominator1 * denominator2 > 0, Errors::invalid_argument(EINVALID_INPUT));
        let gcd: u64 = find_greatest_common_divisor(denominator1, denominator2);
        assert!(gcd > 0, Errors::invalid_argument(Errors::invalid_argument(EINVALID_INPUT)));
        (denominator1 * denominator2) / gcd
    }

    // Create and initialize a shared account 
    public fun initialize(source: &signer, seed: vector<u8>, addresses: vector<address>, numerators: vector<u64>, denominators: vector<u64>): (address) acquires SharedAccount {
        assert!(Vector::length(&addresses) == Vector::length(&numerators), Errors::invalid_argument(EINVALID_INPUT));
        assert!(Vector::length(&addresses) == Vector::length(&denominators), Errors::invalid_argument(EINVALID_INPUT));

        assert!(Account::exists_at(Signer::address_of(source)), Errors::invalid_argument(EACCOUNT_NOT_FOUND));
        let (resource_signer, resource_signer_cap) = Account::create_resource_account(source, seed);
        if (!exists<SharedAccount>(Signer::address_of(&resource_signer))) {
            move_to(
                &resource_signer,
                SharedAccount {
                    share_record: SimpleMap::create(),
                    signer_capability: resource_signer_cap,
                }
            )
        };

        let shared_account = borrow_global_mut<SharedAccount>(Signer::address_of(&resource_signer));

        let i = 0;
        let cumulative_numerator = 0;
        let cumulative_denominator = 1; 

        while (i < Vector::length(&addresses)) {
            let num = *Vector::borrow(&numerators, i);
            let denom = *Vector::borrow(&denominators, i); 
            let addr = *Vector::borrow(&addresses, i);
            assert!(Account::exists_at(addr), Errors::invalid_argument(EACCOUNT_NOT_FOUND));
            assert!(num >= 0 && denom > 0, Errors::invalid_argument(EINVALID_INPUT));

            let common_denom = find_least_common_denominator(cumulative_denominator, denom);
            cumulative_numerator = cumulative_numerator * (common_denom / cumulative_denominator) + num * (common_denom / denom);
            cumulative_denominator = common_denom;

            SimpleMap::add(&mut shared_account.share_record, addr, State { numerator: num, denominator: denom });
            i = i + 1;
        };
        
        assert!(cumulative_numerator == cumulative_denominator, Errors::invalid_argument(EINVALID_NUMERATOR_DENOMINATOR_COMBINATIONS));
        Signer::address_of(&resource_signer)
    }

    // Disperse all available balance to addresses in the shared account
    public(script) fun disperse<CoinType>(resource_addr: address) acquires SharedAccount {
        assert!(exists<SharedAccount>(resource_addr), Errors::invalid_argument(EINVALID_SIGNER));

        let total_balance = Coin::balance<CoinType>(resource_addr);
        assert!(total_balance > 0, Errors::limit_exceeded(EINSUFFICIENT_BALANCE));
                
        let shared_account = borrow_global<SharedAccount>(resource_addr);
        let resource_signer = Account::create_signer_with_capability(&shared_account.signer_capability);

        let i = 0;
        while (i < SimpleMap::length(&shared_account.share_record)) {
            let (key, value) = SimpleMap::get_entry(&shared_account.share_record, i);
            let current_amount = value.numerator * total_balance / value.denominator;
            let current_balance = Coin::balance<CoinType>(resource_addr);
            if (current_amount > current_balance) {
                current_amount = current_balance;
            };

            Coin::transfer<CoinType>(&resource_signer, *key, current_amount);
            i = i + 1;
        };
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113)]
    public fun test_initialize(user: signer, test_user1: signer, test_user2: signer) : (address) acquires SharedAccount {
        let addresses = Vector::empty<address>();
        let numerators = Vector::empty<u64>();
        let denominators = Vector::empty<u64>();
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

        Vector::push_back(&mut denominators, 4);
        Vector::push_back(&mut denominators, 6);

        initialize(&user, seed, addresses, numerators, denominators)
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113)]
    #[expected_failure]
    public fun test_initialize_with_incorrect_numerator_denominator_combo(user: signer, test_user1: signer, test_user2: signer) : (address) acquires SharedAccount {
        let addresses = Vector::empty<address>();
        let numerators = Vector::empty<u64>();
        let denominators = Vector::empty<u64>();
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

        Vector::push_back(&mut denominators, 3);
        Vector::push_back(&mut denominators, 4);

        initialize(&user, seed, addresses, numerators, denominators)
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113, core_resources = @CoreResources, core_framework = @AptosFramework)]
    public(script) fun test_disperse(user: signer, test_user1: signer, test_user2: signer, core_resources: signer, core_framework: signer) acquires SharedAccount {
        use AptosFramework::TestCoin::{Self, TestCoin};

        let user_addr1 = Signer::address_of(&test_user1);
        let user_addr2 = Signer::address_of(&test_user2);
        let resource_addr = test_initialize(user, test_user1, test_user2);
        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);

        let shared_account = borrow_global<SharedAccount>(resource_addr);
        let resource_signer = Account::create_signer_with_capability(&shared_account.signer_capability);
        Coin::register<TestCoin>(&resource_signer);
        TestCoin::mint(&core_framework, resource_addr, 1000);        
        disperse<TestCoin>(resource_addr);
        Coin::destroy_mint_cap<TestCoin>(mint_cap);
        Coin::destroy_burn_cap<TestCoin>(burn_cap);

        assert!(Coin::balance<TestCoin>(user_addr1) == 500, 0);
        assert!(Coin::balance<TestCoin>(user_addr2) == 500, 1);
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113)]
    #[expected_failure]
    public(script) fun test_disperse_insufficient_balance(user: signer, test_user1: signer, test_user2: signer) acquires SharedAccount {
        use AptosFramework::TestCoin::TestCoin;

        let resource_addr = test_initialize(user, test_user1, test_user2);
        let shared_account = borrow_global<SharedAccount>(resource_addr);
        let resource_signer = Account::create_signer_with_capability(&shared_account.signer_capability);
        Coin::register<TestCoin>(&resource_signer);
        disperse<TestCoin>(resource_addr);
    }
}