module AptosFramework::SharedAccount {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::Coin;
    use AptosFramework::IterableTable::{Self, Table};
    use AptosFramework::ResourceAccount;

    struct Shares has store {
        numerator: u128,
        denominator: u128,
    }

    // Resource representing a shared account
    struct SharedAccount has key, store {
        share_record: IterableTable<address, Shares>,
        shared_account_signer: signer,
    }

    const EINVALID_INIT_INPUT: u64 = 0;
    const EACCOUNT_DNE: u64 = 1;
    const EADDRESS_ALREADY_EXISTS: u64 = 2;
    const EINVALID_NUMERATOR_DENOMINATOR_COMBINATIONS: u64 = 3;
    const ENO_SHARE_RECORD = 4;
    const EINVALID_SIGNER = 5;

    public fun initialize_shared_account(user: signer) {
        let user_addr = Signer::address_of(&user);
        create_resource_account(&user, x"01", Vector::empty());
        let container = borrow_global<Container>(user_addr);
        let resource_cap = SimpleMap::borrow(&container.store, &resource_addr);
        let resource_signer = create_signer_with_capability(&resource_cap);

        if (!exists<SharedAccount>(Signer::address_of(resource_signer))) {
            move_to(
                &resource_signer,
                SharedAccount {
                    share_record: IterableTable::new(),
                    shared_account_signer: resource_signer,
                }
            )
        }
    }

    public(script) fun initialize(user: signer, addresses: vector<address>, numerators: vector<u128>, denominator: u128) {
        assert!(Vector::length(&addresses) == Vector::length(&numerators), Errors::invalid_argument(EINVALID_INIT_INPUT));

        initialize_shared_account(user);

        let shared_account = &mut borrow_global_mut<SharedAccount>(account).share_record;

        let sum = 0;
        let i = 0;

        while (i < Vector::length(&numerators)) {
            let num = Vector::borrow(&numerators, i);
            let addr = Vector::borrow(&addresses, i);
            assert!(exists_at(addr), Errors::invalid_argument(EACCOUNT_DNE));
            sum = sum + num;
            i = i + 1;

            IterableTable::add(shared_account, addr, Shares { numerator: num, denominator: denominator });
        }
        assert!(sum == denominator, Errors::invalid_argument(EINVALID_NUMERATOR_DENOMINATOR_COMBINATIONS));
    }

    public(script) fun disperse<CoinType>(resource_signer: signer, total_amount: u128, shared_account: SharedAccount) {
        assert!(IterableTable::length(&shared_account.share_record) > 0,  Errors::invalid_argument(ENO_SHARE_RECORD));

        assert!(resource_signer == shared_account.shared_account_signer, Errors::invalid_argument(EINVALID_SIGNER));

        let current_key = IterableTable::head_key(&shared_account.share_record);
        let tail_key = IterableTable::tail_key(&shared_account.share_record);

        while (true) {
            let record = IterableTable::borrow_iter(&shared_account.share_record, current_key);
            let current_amount = record.val.numerator / record.val.denominator * amount;
            Coin::transfer<CoinType>(&shared_account.shared_account_signer, *current_key, current_amount);

            if (current_key == tail_key) break;

            current_key = record.next;
        }
    }

    #[test]
    public(script) fun test_initialize(user: signer, ) {
        let addresses = Vector::empty();
        let numerators = Vector::empty();

    }
}