module AptosFramework::SharedAccount {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::Coin;
    use AptosFramework::Table::{Self, Table};

    struct Shares has store {
        numerator: u128,
        denominator: u128,
    }

    // Resource representing a shared account
    struct SharedAccount<K, V> has key, store {
        share_record: Table<address, Shares>,
        owner: signer,
    }

    const EINVALID_INIT_INPUT: u64 = 0;
    const EACCOUNT_DNE: u64 = 1;
    const EADDRESS_ALREADY_EXISTS: u64 = 2;
    const EINVALID_NUMERATOR_DENOMINATOR_COMBINATIONS: u64 = 3;

    public fun initialize_shared_account(account: &signer) {
        if (!exists<SharedAccount>(Signer::address_of(account))) {
            move_to(
                account,
                SharedAccount {
                    share_record: Table::new(),
                    owner: account,
                }
            )
        }
    }

    public fun initialize(account: &signer, addresses: vector<address>, numerators: vector<u128>, denominator: u128) {
        assert!(Vector::length(&addresses) == Vector::length(&numerators), Errors::invalid_argument(EINVALID_INIT_INPUT));

        let sum = 0;
        let i = 0;

        initialize_shared_account(account);
        let shared_account = &mut borrow_global_mut<SharedAccount>(account).share_record;

        while (i < Vector::length(&numerators)) {
            let num = *Vector::borrow(&numerators, i);
            let addr = Vector::length(&addresses, i);
            assert!(exists_at(addr), Errors::invalid_argument(EACCOUNT_DNE));
            sum = sum + num;

            Table::add(shared_account, Shares { numerator: num, denominator: denominator });
        }
        assert!(sum == denominator, Errors::invalid_argument(EINVALID_NUMERATOR_DENOMINATOR_COMBINATIONS));
    }

    public fun disperse() {

    }
}