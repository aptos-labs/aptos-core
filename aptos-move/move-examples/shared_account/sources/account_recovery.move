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
    use aptos_framework::resource_account;
    use aptos_framework::account::SignerCapability;

    /// Account recovery struct
    struct AccountRecovery has key, store, drop {
        authorized_addresses: vector<address>,
        required_delay_seconds: u64,
        account_recovery_init: Option<AccountRecoveryInitData>,
        // TODO add recovery capability here.
    }

    // Reverse lookup map
    struct AccountRecoveryReverseLookup has key, store {
        authorized_to_recovery: Table<address, vector<address>>,
    }

    struct AccountRecoveryInitData has key, store {
        recovery_seq_number: u64,
        recovery_initiation_ts: u64,
    }

    struct ModuleData has key {
        // Storing the signer capability here, so the module can programmatically sign for transactions
        signer_cap: SignerCapability,
    }

    fun init_module(signer: &signer) {
        move_to(signer, AccountRecoveryReverseLookup {
            authorized_to_recovery: table::new(),
        });

        let resource_signer_cap = resource_account::retrieve_resource_account_cap(signer, @source_addr);

        move_to(signer, ModuleData {
            signer_cap: resource_signer_cap,
        });
    }

    fun recovery_exists(addr: address): bool {
        exists<AccountRecovery>(addr)
    }

    public fun register(account: &signer,
                        authorized_addresses: vector<address>,
                        required_delay_seconds: u64,
                        rotation_capability_sig_bytes: vector<u8>,
                        account_public_key_bytes: vector<u8>,
    ) acquires AccountRecoveryReverseLookup {
        let addr = signer::address_of(account);

        assert!(!recovery_exists(addr), ERECOVERY_ALREADY_SET);
        assert!(exists<AccountRecoveryReverseLookup>(@source_addr));

        let reverse_lookup = borrow_global_mut<AccountRecoveryReverseLookup>(@source_addr);

        move_to(account, AccountRecovery {
            authorized_addresses,
            required_delay_seconds,
            account_recovery_init: option::none<AccountRecoveryInitData>(),
        });

        let i = 0;
        let len = vector::length(&authorized_addresses);
        while (i < len) {
            let authorized_address = *vector::borrow(&authorized_addresses, i);
            let list = table::borrow_mut_with_default(&mut reverse_lookup.authorized_to_recovery, authorized_address, vector::empty<address>());
            vector::push_back(list, addr);
        };
        account::offer_rotation_capability(signer, rotation_capability_sig_bytes, 0, account_public_key_bytes, @source_addr)
    }

    public fun initiate_account_key_recovery(account: &signer, recovery_address: address) acquires AccountRecovery {
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

    public fun rotate_key(delegated_signer: &signer,
                          recovery_address: address,
                          new_public_key_bytes: vector<u8>,
                          cap_update_table: vector<u8>) acquires AccountRecovery, ModuleData {
        assert!(recovery_exists(recovery_address), ERECOVERY_NOT_SET);

        let account_recovery = borrow_global_mut<AccountRecovery>(recovery_address);
        let delegated_addr = signer::address_of(delegated_signer);
        assert!(vector::contains(&account_recovery.authorized_addresses, &delegated_addr));
        assert!(std::option::is_some(&account_recovery.account_recovery_init));

        let account_recovery_init = std::option::borrow(&account_recovery.account_recovery_init);
        assert!(account::get_sequence_number(recovery_address) == account_recovery_init.recovery_seq_number);
        assert!(timestamp::now_seconds() > account_recovery_init.recovery_initiation_ts + account_recovery.required_delay_seconds);

        let module_data = borrow_global_mut<ModuleData>(@source_addr);
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);

        account::rotate_authentication_key_with_rotation_capability(&resource_signer, recovery_address, 0, new_public_key_bytes, cap_update_table)
    }

    public fun deregister(account: &signer) acquires AccountRecovery, AccountRecoveryReverseLookup {
        let addr = signer::address_of(account);
        let previous = move_from<AccountRecovery>(addr);

        assert!(!exists<AccountRecoveryReverseLookup>(@source_addr));
        account::revoke_rotation_capability(account, @source_addr);

        let reverse_lookup = borrow_global_mut<AccountRecoveryReverseLookup>(@source_addr);

        let i = 0;
        let len = vector::length(&previous.authorized_addresses);
        while (i < len) {
            let authorized_address = *vector::borrow(&previous.authorized_addresses, i);
            let list = table::borrow_mut_with_default(&mut reverse_lookup.authorized_to_recovery, authorized_address, vector::empty<address>());
            let (found, index) = vector::index_of(list, &addr);
            if (found) {
                vector::swap_remove(list, index);
            }
        };
    }
}
