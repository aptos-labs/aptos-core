module SharedAccount::SharedAccount {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::Account;
    use AptosFramework::Coin;

    // struct State records the address of the share_holder and the numerator and the denominator of a fraction
    struct ShareRecord has store {
        share_holder: address,
        numerator: u64,
    }

    // Resource representing a shared account
    struct SharedAccount has key {
        share_record: vector<ShareRecord>,
        total_shares: u64,
        signer_capability: Account::SignerCapability,
    }

    const EACCOUNT_NOT_FOUND: u64 = 0;
    const ERESOURCE_DNE: u64 = 1;
    const EINSUFFICIENT_BALANCE: u64 = 2;

    public(script) fun initialize(source: &signer, seed: vector<u8>, addresses: vector<address>, numerators: vector<u64>) {
        initialize_internal(source, seed, addresses, numerators);
    }

    // Create and initialize a shared account
    fun initialize_internal(source: &signer, seed: vector<u8>, addresses: vector<address>, numerators: vector<u64>): address {
        let i = 0;
        let total = 0;
        let share_record = Vector::empty<ShareRecord>();

        while (i < Vector::length(&addresses)) {
            let num_shares = *Vector::borrow(&numerators, i);
            let addr = *Vector::borrow(&addresses, i);
            assert!(Account::exists_at(addr), Errors::invalid_argument(EACCOUNT_NOT_FOUND));

            Vector::push_back(&mut share_record, ShareRecord { share_holder: addr, numerator: num_shares });
            total = total + num_shares;
            i = i + 1;
        };

        let (resource_signer, resource_signer_cap) = Account::create_resource_account(source, seed);
        move_to(
            &resource_signer,
            SharedAccount {
                share_record,
                total_shares: total,
                signer_capability: resource_signer_cap,
            }
        );

        Signer::address_of(&resource_signer)
    }

    // Disperse all available balance to addresses in the shared account
    public(script) fun disperse<CoinType>(resource_addr: address) acquires SharedAccount {
        assert!(exists<SharedAccount>(resource_addr), Errors::invalid_argument(ERESOURCE_DNE));

        let total_balance = Coin::balance<CoinType>(resource_addr);
        assert!(total_balance > 0, Errors::limit_exceeded(EINSUFFICIENT_BALANCE));

        let shared_account = borrow_global<SharedAccount>(resource_addr);
        let resource_signer = Account::create_signer_with_capability(&shared_account.signer_capability);

        let i = 0;
        let current_balance = Coin::balance<CoinType>(resource_addr);

        while (i < Vector::length(&shared_account.share_record)) {
            let share_record = Vector::borrow(&shared_account.share_record, i);
            let current_amount = share_record.numerator * total_balance / shared_account.total_shares;
            if (current_amount > current_balance) {
                current_amount = current_balance;
            };
            Coin::transfer<CoinType>(&resource_signer, share_record.share_holder, current_amount);

            current_balance = current_balance - current_amount;
            i = i + 1;
        };
    }

    #[test_only]
    public(script) fun set_up(user: signer, test_user1: signer, test_user2: signer) : address {
        let addresses = Vector::empty<address>();
        let numerators = Vector::empty<u64>();
        let seed = x"01";
        let user_addr = Signer::address_of(&user);
        let user_addr1 = Signer::address_of(&test_user1);
        let user_addr2 = Signer::address_of(&test_user2);

        Account::create_account(user_addr);
        Account::create_account(user_addr1);
        Account::create_account(user_addr2);

        Vector::push_back(&mut addresses, user_addr1);
        Vector::push_back(&mut addresses, user_addr2);

        Vector::push_back(&mut numerators, 1);
        Vector::push_back(&mut numerators, 4);

        initialize_internal(&user, seed, addresses, numerators)
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113, core_resources = @CoreResources, core_framework = @AptosFramework)]
    public(script) fun test_disperse(user: signer, test_user1: signer, test_user2: signer, core_resources: signer, core_framework: signer) acquires SharedAccount {
        use AptosFramework::TestCoin::{Self, TestCoin};
        let user_addr1 = Signer::address_of(&test_user1);
        let user_addr2 = Signer::address_of(&test_user2);
        let resource_addr = set_up(user, test_user1, test_user2);
        let (mint_cap, burn_cap) = TestCoin::initialize(&core_framework, &core_resources);

        let shared_account = borrow_global<SharedAccount>(resource_addr);
        let resource_signer = Account::create_signer_with_capability(&shared_account.signer_capability);
        Coin::register<TestCoin>(&resource_signer);
        TestCoin::mint(&core_framework, resource_addr, 1000);
        disperse<TestCoin>(resource_addr);
        Coin::destroy_mint_cap<TestCoin>(mint_cap);
        Coin::destroy_burn_cap<TestCoin>(burn_cap);

        assert!(Coin::balance<TestCoin>(user_addr1) == 200, 0);
        assert!(Coin::balance<TestCoin>(user_addr2) == 800, 1);
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113)]
    #[expected_failure]
    public(script) fun test_disperse_insufficient_balance(user: signer, test_user1: signer, test_user2: signer) acquires SharedAccount {
        use AptosFramework::TestCoin::TestCoin;

        let resource_addr = set_up(user, test_user1, test_user2);
        let shared_account = borrow_global<SharedAccount>(resource_addr);
        let resource_signer = Account::create_signer_with_capability(&shared_account.signer_capability);
        Coin::register<TestCoin>(&resource_signer);
        disperse<TestCoin>(resource_addr);
    }
}
