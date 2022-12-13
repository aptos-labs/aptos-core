module aptos_framework::account_recovery {

    const ERECOVERY_NOT_SET: u64 = 1;
    const ERECOVERY_ALREADY_SET: u64 = 2;
    const EAUI: u64 = 2;

    use std::signer;
    use aptos_std::table::{Table, add};
    use aptos_framework::account;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_std::table;
    use std::vector;
    use std::option::Option;
    use std::option;

    /// Account recovery struct
    struct AccountRecovery has key, store, drop {
        authorized_addresses: vector<address>,
        required_delay_seconds: u64,
        account_recovery_init: Option<AccountRecoveryInitData>,
        // TODO add recovery capability here.
    }

    // Reverse lookup map
    struct AccountRecoveryReverseLookup has key, store {
        can_recover_accounts: Table<address, vector<address>>,
    }

    struct AccountRecoveryInitData has key, store {
        recovery_seq_number: u64,
        recovery_initiation_ts: u64,
    }

    fun recovery_exists(addr: address): bool {
        exists<AccountRecovery>(addr)
    }

    public fun register(account: &signer, authorized_addresses: vector<address>, required_delay: u64) acquires AccountRecovery, AccountRecoveryReverseLookup {
        let addr = signer::address_of(account);

        assert!(!recovery_exists(addr), ERECOVERY_ALREADY_SET);

        move_to(account, AccountRecovery {
            authorized_addresses,
            required_delay_seconds,
            account_recovery_init: option::none<>(),
        });

        let i = 0;
        let len = vector::length(&authorized_addresses);
        while (i < len) {
            let authorized_address = *vector::borrow(&authorized_addresses, i);
            if (exists<AccountRecoveryReverseLookup>(authorized_address)) {
                let reverse_lookup = borrow_global_mut<AccountRecoveryReverseLookup>(authorized_address);
                vector::push_back(&mut reverse_lookup.can_recover_accounts, addr);
            } else {
                move_to(&authorized_address, AccountRecoveryReverseLookup {
                   can_recover_accounts: vector::singleton(addr),
                });
            };
        };
    }

    public fun initiate_account_key_recovery(account: &signer, recovery_address: address) acquires AccountRecovery, AddressToAccountRecovery {
        assert!(recovery_exists(recovery_address), ERECOVERY_NOT_SET);

        let account_recovery = borrow_global_mut<AccountRecovery>(recovery_address);
        let addr = signer::address_of(account);
        assert!(vector::contains(&account_recovery.authorized_addresses, &addr));
        assert!(account_recovery.account_recovery_init);

        account_recovery.account_recovery_init = std::option::some(AccountRecoveryInitData{
            recovery_seq_number: account::get_sequence_number(recovery_address),
            recovery_initiation_ts: timestamp::now_seconds(),
        });
    }

    public fun rotate_key(account: &signer, recovery_address: address) acquires AccountRecovery {
        assert!(recovery_exists(recovery_address), ERECOVERY_NOT_SET);

        let account_recovery = borrow_global_mut<AccountRecovery>(recovery_address);
        let addr = signer::address_of(account);
        assert!(vector::contains(&account_recovery.authorized_addresses, &addr));
        assert!(std::option::is_some(&account_recovery.account_recovery_init));

        let account_recovery_init = std::option::borrow(&account_recovery.account_recovery_init);
        assert!(account::get_sequence_number(recovery_address) == account_recovery_init.recovery_seq_number);
        assert!(timestamp::now_seconds() > account_recovery_init.recovery_initiation_ts + account_recovery.required_delay_seconds);


    }

    public fun deregister(account: &signer) acquires AccountRecovery {
        let addr = signer::address_of(account);
        let previous = move_from<AccountRecovery>(addr);

        let i = 0;
        let len = vector::length(&previous.authorized_addresses);
        while (i < len) {
            let cur = vector::borrow(&previous.authorized_addresses, i);

        }
    }

    public fun update_recovery() {

    }

}