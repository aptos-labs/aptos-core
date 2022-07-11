module AptosFramework::SharedAccount {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::Account;
    use AptosFramework::Coin;
    use AptosFramework::SimpleMap::{Self, SimpleMap};
    use AptosFramework::ResourceAccount;

    struct Shares has store {
        numerator: u128,
        denominator: u128,
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

    public(script) fun initialize_shared_account(source: &signer, seed: vector<u8>): (signer) {
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

    public(script) fun initialize(source: &signer, seed: vector<u8>, addresses: vector<address>, numerators: vector<u128>, denominator: u128): (signer) acquires SharedAccount {
        assert!(Vector::length(&addresses) == Vector::length(&numerators), Errors::invalid_argument(EINVALID_INIT_INPUT));

        let resource_signer = initialize_shared_account(source, seed);

        let shared_account = borrow_global_mut<SharedAccount>(Signer::address_of(&resource_signer));

        let sum: u128 = 0;
        let i = 0;

        while (i < Vector::length(&numerators)) {
            let num = *Vector::borrow(&numerators, i);
            let addr = *Vector::borrow(&addresses, i);
            // assert!(Account::exists_at(addr), Errors::invalid_argument(EACCOUNT_DNE));
            sum = sum + num;
            i = i + 1;

            SimpleMap::add(&mut shared_account.share_record, addr, Shares { numerator: num, denominator: denominator });
        };
        assert!(sum == denominator, Errors::invalid_argument(EINVALID_NUMERATOR_DENOMINATOR_COMBINATIONS));
        resource_signer
    }

    public(script) fun disperse<CoinType>(resource_signer: &signer, total_amount: u128) acquires SharedAccount {
        assert!(exists<SharedAccount>(Signer::address_of(resource_signer)), Errors::invalid_argument(EINVALID_SIGNER));
                
        let i = 0;

        let shared_account = borrow_global<SharedAccount>(Signer::address_of(resource_signer));

        while (i < SimpleMap::length(&shared_account.share_record)) {
            let (key, value) = SimpleMap::get_entry(&shared_account.share_record, i);
            let current_amount = value.numerator / value.denominator * total_amount;
            Coin::transfer<CoinType>(resource_signer, *key, (current_amount as u64));
            i = i + 1;
        };
    }

    #[test(user = @0x1111, test_user1 = @0x1112, test_user2 = @0x1113)]
    public(script) fun test_initialize(user: signer, test_user1: signer, test_user2: signer) acquires SharedAccount {
        use Std::BCS;
        use Std::Hash;

        let addresses = Vector::empty<address>();
        let numerators = Vector::empty<u128>();
        let denominator: u128 = 5;
        let seed = x"01";
        let user_addr1 = Signer::address_of(&test_user1);
        let user_addr2 = Signer::address_of(&test_user2);
        let bytes = BCS::to_bytes(&user_addr1);
        let bytes2 = BCS::to_bytes(&user_addr2);
        
        Vector::append(&mut bytes, copy seed);
        Vector::append(&mut bytes, copy seed);
        let addr1 = Account::create_address_for_test(Hash::sha3_256(bytes));
        let addr2 = Account::create_address_for_test(Hash::sha3_256(bytes2));


        Vector::push_back(&mut addresses, addr1);
        Vector::push_back(&mut addresses, addr2);

        Vector::push_back(&mut numerators, 2);
        Vector::push_back(&mut numerators, 3);

        initialize(&user, seed, addresses, numerators, denominator);
    }
}