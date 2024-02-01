module 0x1::multisig_account {
    struct AddOwnersEvent has drop, store {
        owners_added: vector<address>,
    }
    
    struct CreateTransactionEvent has drop, store {
        creator: address,
        sequence_number: u64,
        transaction: MultisigTransaction,
    }
    
    struct ExecuteRejectedTransactionEvent has drop, store {
        sequence_number: u64,
        num_rejections: u64,
        executor: address,
    }
    
    struct ExecutionError has copy, drop, store {
        abort_location: 0x1::string::String,
        error_type: 0x1::string::String,
        error_code: u64,
    }
    
    struct MetadataUpdatedEvent has drop, store {
        old_metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
        new_metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
    }
    
    struct MultisigAccount has key {
        owners: vector<address>,
        num_signatures_required: u64,
        transactions: 0x1::table::Table<u64, MultisigTransaction>,
        last_executed_sequence_number: u64,
        next_sequence_number: u64,
        signer_cap: 0x1::option::Option<0x1::account::SignerCapability>,
        metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
        add_owners_events: 0x1::event::EventHandle<AddOwnersEvent>,
        remove_owners_events: 0x1::event::EventHandle<RemoveOwnersEvent>,
        update_signature_required_events: 0x1::event::EventHandle<UpdateSignaturesRequiredEvent>,
        create_transaction_events: 0x1::event::EventHandle<CreateTransactionEvent>,
        vote_events: 0x1::event::EventHandle<VoteEvent>,
        execute_rejected_transaction_events: 0x1::event::EventHandle<ExecuteRejectedTransactionEvent>,
        execute_transaction_events: 0x1::event::EventHandle<TransactionExecutionSucceededEvent>,
        transaction_execution_failed_events: 0x1::event::EventHandle<TransactionExecutionFailedEvent>,
        metadata_updated_events: 0x1::event::EventHandle<MetadataUpdatedEvent>,
    }
    
    struct MultisigAccountCreationMessage has copy, drop {
        chain_id: u8,
        account_address: address,
        sequence_number: u64,
        owners: vector<address>,
        num_signatures_required: u64,
    }
    
    struct MultisigAccountCreationWithAuthKeyRevocationMessage has copy, drop {
        chain_id: u8,
        account_address: address,
        sequence_number: u64,
        owners: vector<address>,
        num_signatures_required: u64,
    }
    
    struct MultisigTransaction has copy, drop, store {
        payload: 0x1::option::Option<vector<u8>>,
        payload_hash: 0x1::option::Option<vector<u8>>,
        votes: 0x1::simple_map::SimpleMap<address, bool>,
        creator: address,
        creation_time_secs: u64,
    }
    
    struct RemoveOwnersEvent has drop, store {
        owners_removed: vector<address>,
    }
    
    struct TransactionExecutionFailedEvent has drop, store {
        executor: address,
        sequence_number: u64,
        transaction_payload: vector<u8>,
        num_approvals: u64,
        execution_error: ExecutionError,
    }
    
    struct TransactionExecutionSucceededEvent has drop, store {
        executor: address,
        sequence_number: u64,
        transaction_payload: vector<u8>,
        num_approvals: u64,
    }
    
    struct UpdateSignaturesRequiredEvent has drop, store {
        old_num_signatures_required: u64,
        new_num_signatures_required: u64,
    }
    
    struct VoteEvent has drop, store {
        owner: address,
        sequence_number: u64,
        approved: bool,
    }
    
    public entry fun create(arg0: &signer, arg1: u64, arg2: vector<0x1::string::String>, arg3: vector<vector<u8>>) acquires MultisigAccount {
        create_with_owners(arg0, vector[], arg1, arg2, arg3);
    }
    
    entry fun add_owner(arg0: &signer, arg1: address) acquires MultisigAccount {
        let v0 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v0, arg1);
        add_owners(arg0, v0);
    }
    
    entry fun add_owners(arg0: &signer, arg1: vector<address>) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), arg1, vector[], 0x1::option::none<u64>());
    }
    
    entry fun add_owners_and_update_signatures_required(arg0: &signer, arg1: vector<address>, arg2: u64) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), arg1, vector[], 0x1::option::some<u64>(arg2));
    }
    
    fun add_transaction(arg0: address, arg1: &mut MultisigAccount, arg2: MultisigTransaction) {
        0x1::simple_map::add<address, bool>(&mut arg2.votes, arg0, true);
        let v0 = arg1.next_sequence_number;
        arg1.next_sequence_number = v0 + 1;
        0x1::table::add<u64, MultisigTransaction>(&mut arg1.transactions, v0, arg2);
        let v1 = CreateTransactionEvent{
            creator         : arg0, 
            sequence_number : v0, 
            transaction     : arg2,
        };
        0x1::event::emit_event<CreateTransactionEvent>(&mut arg1.create_transaction_events, v1);
    }
    
    public entry fun approve_transaction(arg0: &signer, arg1: address, arg2: u64) acquires MultisigAccount {
        vote_transanction(arg0, arg1, arg2, true);
    }
    
    fun assert_is_owner(arg0: &signer, arg1: &MultisigAccount) {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(0x1::vector::contains<address>(&arg1.owners, &v0), 0x1::error::permission_denied(2003));
    }
    
    fun assert_multisig_account_exists(arg0: address) {
        assert!(exists<MultisigAccount>(arg0), 0x1::error::invalid_state(2002));
    }
    
    public fun can_be_executed(arg0: address, arg1: u64) : bool acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg0);
        assert!(arg1 > 0 && arg1 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        let v1 = 0x1::table::borrow<u64, MultisigTransaction>(&v0.transactions, arg1);
        let (v2, _) = num_approvals_and_rejections(&v0.owners, v1);
        arg1 == v0.last_executed_sequence_number + 1 && v2 >= v0.num_signatures_required
    }
    
    public fun can_be_rejected(arg0: address, arg1: u64) : bool acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg0);
        assert!(arg1 > 0 && arg1 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        let v1 = 0x1::table::borrow<u64, MultisigTransaction>(&v0.transactions, arg1);
        let (_, v3) = num_approvals_and_rejections(&v0.owners, v1);
        arg1 == v0.last_executed_sequence_number + 1 && v3 >= v0.num_signatures_required
    }
    
    fun create_multisig_account(arg0: &signer) : (signer, 0x1::account::SignerCapability) {
        let v0 = 0x1::account::get_sequence_number(0x1::signer::address_of(arg0));
        let v1 = create_multisig_account_seed(0x1::bcs::to_bytes<u64>(&v0));
        let (v2, v3) = 0x1::account::create_resource_account(arg0, v1);
        let v4 = v2;
        if (!0x1::coin::is_account_registered<0x1::aptos_coin::AptosCoin>(0x1::signer::address_of(&v4))) {
            0x1::coin::register<0x1::aptos_coin::AptosCoin>(&v4);
        };
        (v4, v3)
    }
    
    fun create_multisig_account_seed(arg0: vector<u8>) : vector<u8> {
        let v0 = 0x1::vector::empty<u8>();
        0x1::vector::append<u8>(&mut v0, b"aptos_framework::multisig_account");
        0x1::vector::append<u8>(&mut v0, arg0);
        v0
    }
    
    public entry fun create_transaction(arg0: &signer, arg1: address, arg2: vector<u8>) acquires MultisigAccount {
        assert!(0x1::vector::length<u8>(&arg2) > 0, 0x1::error::invalid_argument(4));
        assert_multisig_account_exists(arg1);
        let v0 = borrow_global_mut<MultisigAccount>(arg1);
        assert_is_owner(arg0, v0);
        let v1 = 0x1::signer::address_of(arg0);
        let v2 = 0x1::option::some<vector<u8>>(arg2);
        let v3 = 0x1::option::none<vector<u8>>();
        let v4 = 0x1::simple_map::create<address, bool>();
        let v5 = 0x1::timestamp::now_seconds();
        let v6 = MultisigTransaction{
            payload            : v2, 
            payload_hash       : v3, 
            votes              : v4, 
            creator            : v1, 
            creation_time_secs : v5,
        };
        add_transaction(v1, v0, v6);
    }
    
    public entry fun create_transaction_with_hash(arg0: &signer, arg1: address, arg2: vector<u8>) acquires MultisigAccount {
        assert!(0x1::vector::length<u8>(&arg2) == 32, 0x1::error::invalid_argument(12));
        assert_multisig_account_exists(arg1);
        let v0 = borrow_global_mut<MultisigAccount>(arg1);
        assert_is_owner(arg0, v0);
        let v1 = 0x1::signer::address_of(arg0);
        let v2 = 0x1::option::none<vector<u8>>();
        let v3 = 0x1::option::some<vector<u8>>(arg2);
        let v4 = 0x1::simple_map::create<address, bool>();
        let v5 = 0x1::timestamp::now_seconds();
        let v6 = MultisigTransaction{
            payload            : v2, 
            payload_hash       : v3, 
            votes              : v4, 
            creator            : v1, 
            creation_time_secs : v5,
        };
        add_transaction(v1, v0, v6);
    }
    
    public entry fun create_with_existing_account(arg0: address, arg1: vector<address>, arg2: u64, arg3: u8, arg4: vector<u8>, arg5: vector<u8>, arg6: vector<0x1::string::String>, arg7: vector<vector<u8>>) acquires MultisigAccount {
        let v0 = 0x1::chain_id::get();
        let v1 = 0x1::account::get_sequence_number(arg0);
        let v2 = MultisigAccountCreationMessage{
            chain_id                : v0, 
            account_address         : arg0, 
            sequence_number         : v1, 
            owners                  : arg1, 
            num_signatures_required : arg2,
        };
        0x1::account::verify_signed_message<MultisigAccountCreationMessage>(arg0, arg3, arg4, arg5, v2);
        let v3 = 0x1::create_signer::create_signer(arg0);
        create_with_owners_internal(&v3, arg1, arg2, 0x1::option::none<0x1::account::SignerCapability>(), arg6, arg7);
    }
    
    public entry fun create_with_existing_account_and_revoke_auth_key(arg0: address, arg1: vector<address>, arg2: u64, arg3: u8, arg4: vector<u8>, arg5: vector<u8>, arg6: vector<0x1::string::String>, arg7: vector<vector<u8>>) acquires MultisigAccount {
        let v0 = 0x1::chain_id::get();
        let v1 = 0x1::account::get_sequence_number(arg0);
        let v2 = MultisigAccountCreationWithAuthKeyRevocationMessage{
            chain_id                : v0, 
            account_address         : arg0, 
            sequence_number         : v1, 
            owners                  : arg1, 
            num_signatures_required : arg2,
        };
        0x1::account::verify_signed_message<MultisigAccountCreationWithAuthKeyRevocationMessage>(arg0, arg3, arg4, arg5, v2);
        let v3 = 0x1::create_signer::create_signer(arg0);
        let v4 = &v3;
        create_with_owners_internal(v4, arg1, arg2, 0x1::option::none<0x1::account::SignerCapability>(), arg6, arg7);
        let v5 = 0x1::signer::address_of(v4);
        0x1::account::rotate_authentication_key_internal(v4, x"0000000000000000000000000000000000000000000000000000000000000000");
        if (0x1::account::is_signer_capability_offered(v5)) {
            0x1::account::revoke_any_signer_capability(v4);
        };
        if (0x1::account::is_rotation_capability_offered(v5)) {
            0x1::account::revoke_any_rotation_capability(v4);
        };
    }
    
    public entry fun create_with_owners(arg0: &signer, arg1: vector<address>, arg2: u64, arg3: vector<0x1::string::String>, arg4: vector<vector<u8>>) acquires MultisigAccount {
        let (v0, v1) = create_multisig_account(arg0);
        let v2 = v0;
        0x1::vector::push_back<address>(&mut arg1, 0x1::signer::address_of(arg0));
        let v3 = 0x1::option::some<0x1::account::SignerCapability>(v1);
        create_with_owners_internal(&v2, arg1, arg2, v3, arg3, arg4);
    }
    
    fun create_with_owners_internal(arg0: &signer, arg1: vector<address>, arg2: u64, arg3: 0x1::option::Option<0x1::account::SignerCapability>, arg4: vector<0x1::string::String>, arg5: vector<vector<u8>>) acquires MultisigAccount {
        assert!(0x1::features::multisig_accounts_enabled(), 0x1::error::unavailable(14));
        assert!(arg2 > 0 && arg2 <= 0x1::vector::length<address>(&arg1), 0x1::error::invalid_argument(11));
        validate_owners(&arg1, 0x1::signer::address_of(arg0));
        let v0 = arg1;
        let v1 = 0x1::table::new<u64, MultisigTransaction>();
        let v2 = 0x1::simple_map::create<0x1::string::String, vector<u8>>();
        let v3 = 0x1::account::new_event_handle<AddOwnersEvent>(arg0);
        let v4 = 0x1::account::new_event_handle<RemoveOwnersEvent>(arg0);
        let v5 = 0x1::account::new_event_handle<UpdateSignaturesRequiredEvent>(arg0);
        let v6 = 0x1::account::new_event_handle<CreateTransactionEvent>(arg0);
        let v7 = 0x1::account::new_event_handle<VoteEvent>(arg0);
        let v8 = 0x1::account::new_event_handle<ExecuteRejectedTransactionEvent>(arg0);
        let v9 = 0x1::account::new_event_handle<TransactionExecutionSucceededEvent>(arg0);
        let v10 = 0x1::account::new_event_handle<TransactionExecutionFailedEvent>(arg0);
        let v11 = 0x1::account::new_event_handle<MetadataUpdatedEvent>(arg0);
        let v12 = MultisigAccount{
            owners                              : v0, 
            num_signatures_required             : arg2, 
            transactions                        : v1, 
            last_executed_sequence_number       : 0, 
            next_sequence_number                : 1, 
            signer_cap                          : arg3, 
            metadata                            : v2, 
            add_owners_events                   : v3, 
            remove_owners_events                : v4, 
            update_signature_required_events    : v5, 
            create_transaction_events           : v6, 
            vote_events                         : v7, 
            execute_rejected_transaction_events : v8, 
            execute_transaction_events          : v9, 
            transaction_execution_failed_events : v10, 
            metadata_updated_events             : v11,
        };
        move_to<MultisigAccount>(arg0, v12);
        update_metadata_internal(arg0, arg4, arg5, false);
    }
    
    public entry fun create_with_owners_then_remove_bootstrapper(arg0: &signer, arg1: vector<address>, arg2: u64, arg3: vector<0x1::string::String>, arg4: vector<vector<u8>>) acquires MultisigAccount {
        let v0 = 0x1::signer::address_of(arg0);
        create_with_owners(arg0, arg1, arg2, arg3, arg4);
        let v1 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v1, v0);
        update_owner_schema(get_next_multisig_account_address(v0), vector[], v1, 0x1::option::none<u64>());
    }
    
    public entry fun execute_rejected_transaction(arg0: &signer, arg1: address) acquires MultisigAccount {
        assert_multisig_account_exists(arg1);
        let v0 = borrow_global_mut<MultisigAccount>(arg1);
        assert_is_owner(arg0, v0);
        let v1 = v0.last_executed_sequence_number + 1;
        let v2 = 0x1::table::contains<u64, MultisigTransaction>(&v0.transactions, v1);
        assert!(v2, 0x1::error::not_found(2006));
        let (_, v4) = remove_executed_transaction(v0);
        assert!(v4 >= v0.num_signatures_required, 0x1::error::invalid_state(10));
        let v5 = &mut v0.execute_rejected_transaction_events;
        let v6 = 0x1::signer::address_of(arg0);
        let v7 = ExecuteRejectedTransactionEvent{
            sequence_number : v1, 
            num_rejections  : v4, 
            executor        : v6,
        };
        0x1::event::emit_event<ExecuteRejectedTransactionEvent>(v5, v7);
    }
    
    fun failed_transaction_execution_cleanup(arg0: address, arg1: address, arg2: vector<u8>, arg3: ExecutionError) acquires MultisigAccount {
        let v0 = borrow_global_mut<MultisigAccount>(arg1);
        let (v1, _) = remove_executed_transaction(v0);
        let v3 = v0.last_executed_sequence_number;
        let v4 = TransactionExecutionFailedEvent{
            executor            : arg0, 
            sequence_number     : v3, 
            transaction_payload : arg2, 
            num_approvals       : v1, 
            execution_error     : arg3,
        };
        0x1::event::emit_event<TransactionExecutionFailedEvent>(&mut v0.transaction_execution_failed_events, v4);
    }
    
    public fun get_next_multisig_account_address(arg0: address) : address {
        let v0 = 0x1::account::get_sequence_number(arg0);
        let v1 = create_multisig_account_seed(0x1::bcs::to_bytes<u64>(&v0));
        0x1::account::create_resource_address(&arg0, v1)
    }
    
    public fun get_next_transaction_payload(arg0: address, arg1: vector<u8>) : vector<u8> acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg0);
        let v1 = 0x1::table::borrow<u64, MultisigTransaction>(&v0.transactions, v0.last_executed_sequence_number + 1);
        if (0x1::option::is_some<vector<u8>>(&v1.payload)) {
            *0x1::option::borrow<vector<u8>>(&v1.payload)
        } else {
            arg1
        }
    }
    
    public fun get_pending_transactions(arg0: address) : vector<MultisigTransaction> acquires MultisigAccount {
        let v0 = 0x1::vector::empty<MultisigTransaction>();
        let v1 = borrow_global<MultisigAccount>(arg0);
        let v2 = v1.last_executed_sequence_number + 1;
        while (v2 < v1.next_sequence_number) {
            let v3 = *0x1::table::borrow<u64, MultisigTransaction>(&v1.transactions, v2);
            0x1::vector::push_back<MultisigTransaction>(&mut v0, v3);
            v2 = v2 + 1;
        };
        v0
    }
    
    public fun get_transaction(arg0: address, arg1: u64) : MultisigTransaction acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg0);
        assert!(arg1 > 0 && arg1 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        *0x1::table::borrow<u64, MultisigTransaction>(&v0.transactions, arg1)
    }
    
    public fun last_resolved_sequence_number(arg0: address) : u64 acquires MultisigAccount {
        borrow_global_mut<MultisigAccount>(arg0).last_executed_sequence_number
    }
    
    public fun metadata(arg0: address) : 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>> acquires MultisigAccount {
        borrow_global<MultisigAccount>(arg0).metadata
    }
    
    public fun next_sequence_number(arg0: address) : u64 acquires MultisigAccount {
        borrow_global_mut<MultisigAccount>(arg0).next_sequence_number
    }
    
    fun num_approvals_and_rejections(arg0: &vector<address>, arg1: &MultisigTransaction) : (u64, u64) {
        let v0 = 0;
        let v1 = 0;
        let v2 = &arg1.votes;
        let v3 = 0;
        while (v3 < 0x1::vector::length<address>(arg0)) {
            let v4 = 0x1::vector::borrow<address>(arg0, v3);
            if (0x1::simple_map::contains_key<address, bool>(v2, v4)) {
                if (*0x1::simple_map::borrow<address, bool>(v2, v4)) {
                    v0 = v0 + 1;
                } else {
                    v1 = v1 + 1;
                };
            };
            v3 = v3 + 1;
        };
        (v0, v1)
    }
    
    public fun num_signatures_required(arg0: address) : u64 acquires MultisigAccount {
        borrow_global<MultisigAccount>(arg0).num_signatures_required
    }
    
    public fun owners(arg0: address) : vector<address> acquires MultisigAccount {
        borrow_global<MultisigAccount>(arg0).owners
    }
    
    public entry fun reject_transaction(arg0: &signer, arg1: address, arg2: u64) acquires MultisigAccount {
        vote_transanction(arg0, arg1, arg2, false);
    }
    
    fun remove_executed_transaction(arg0: &mut MultisigAccount) : (u64, u64) {
        let v0 = arg0.last_executed_sequence_number + 1;
        let v1 = 0x1::table::remove<u64, MultisigTransaction>(&mut arg0.transactions, v0);
        arg0.last_executed_sequence_number = v0;
        num_approvals_and_rejections(&arg0.owners, &v1)
    }
    
    entry fun remove_owner(arg0: &signer, arg1: address) acquires MultisigAccount {
        let v0 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v0, arg1);
        remove_owners(arg0, v0);
    }
    
    entry fun remove_owners(arg0: &signer, arg1: vector<address>) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), vector[], arg1, 0x1::option::none<u64>());
    }
    
    fun successful_transaction_execution_cleanup(arg0: address, arg1: address, arg2: vector<u8>) acquires MultisigAccount {
        let v0 = borrow_global_mut<MultisigAccount>(arg1);
        let (v1, _) = remove_executed_transaction(v0);
        let v3 = v0.last_executed_sequence_number;
        let v4 = TransactionExecutionSucceededEvent{
            executor            : arg0, 
            sequence_number     : v3, 
            transaction_payload : arg2, 
            num_approvals       : v1,
        };
        0x1::event::emit_event<TransactionExecutionSucceededEvent>(&mut v0.execute_transaction_events, v4);
    }
    
    entry fun swap_owner(arg0: &signer, arg1: address, arg2: address) acquires MultisigAccount {
        let v0 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v0, arg1);
        let v1 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v1, arg2);
        update_owner_schema(0x1::signer::address_of(arg0), v0, v1, 0x1::option::none<u64>());
    }
    
    entry fun swap_owners(arg0: &signer, arg1: vector<address>, arg2: vector<address>) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), arg1, arg2, 0x1::option::none<u64>());
    }
    
    entry fun swap_owners_and_update_signatures_required(arg0: &signer, arg1: vector<address>, arg2: vector<address>, arg3: u64) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), arg1, arg2, 0x1::option::some<u64>(arg3));
    }
    
    entry fun update_metadata(arg0: &signer, arg1: vector<0x1::string::String>, arg2: vector<vector<u8>>) acquires MultisigAccount {
        update_metadata_internal(arg0, arg1, arg2, true);
    }
    
    fun update_metadata_internal(arg0: &signer, arg1: vector<0x1::string::String>, arg2: vector<vector<u8>>, arg3: bool) acquires MultisigAccount {
        let v0 = 0x1::vector::length<0x1::string::String>(&arg1);
        assert!(v0 == 0x1::vector::length<vector<u8>>(&arg2), 0x1::error::invalid_argument(15));
        let v1 = 0x1::signer::address_of(arg0);
        assert_multisig_account_exists(v1);
        let v2 = borrow_global_mut<MultisigAccount>(v1);
        v2.metadata = 0x1::simple_map::create<0x1::string::String, vector<u8>>();
        let v3 = &mut v2.metadata;
        let v4 = 0;
        while (v4 < v0) {
            let v5 = *0x1::vector::borrow<0x1::string::String>(&arg1, v4);
            let v6 = *0x1::vector::borrow<vector<u8>>(&arg2, v4);
            let v7 = !0x1::simple_map::contains_key<0x1::string::String, vector<u8>>(v3, &v5);
            assert!(v7, 0x1::error::invalid_argument(16));
            0x1::simple_map::add<0x1::string::String, vector<u8>>(v3, v5, v6);
            v4 = v4 + 1;
        };
        if (arg3) {
            let v8 = MetadataUpdatedEvent{
                old_metadata : v2.metadata, 
                new_metadata : v2.metadata,
            };
            0x1::event::emit_event<MetadataUpdatedEvent>(&mut v2.metadata_updated_events, v8);
        };
    }
    
    fun update_owner_schema(arg0: address, arg1: vector<address>, arg2: vector<address>, arg3: 0x1::option::Option<u64>) acquires MultisigAccount {
        assert_multisig_account_exists(arg0);
        let v0 = borrow_global_mut<MultisigAccount>(arg0);
        let v1 = &arg1;
        let v2 = 0;
        while (v2 < 0x1::vector::length<address>(v1)) {
            let v3 = !0x1::vector::contains<address>(&arg2, 0x1::vector::borrow<address>(v1, v2));
            assert!(v3, 0x1::error::invalid_argument(18));
            v2 = v2 + 1;
        };
        if (0x1::vector::length<address>(&arg1) > 0) {
            0x1::vector::append<address>(&mut v0.owners, arg1);
            validate_owners(&v0.owners, arg0);
            let v4 = AddOwnersEvent{owners_added: arg1};
            0x1::event::emit_event<AddOwnersEvent>(&mut v0.add_owners_events, v4);
        };
        if (0x1::vector::length<address>(&arg2) > 0) {
            let v5 = &mut v0.owners;
            let v6 = vector[];
            let v7 = &arg2;
            let v8 = 0;
            while (v8 < 0x1::vector::length<address>(v7)) {
                let (v9, v10) = 0x1::vector::index_of<address>(v5, 0x1::vector::borrow<address>(v7, v8));
                if (v9) {
                    0x1::vector::push_back<address>(&mut v6, 0x1::vector::swap_remove<address>(v5, v10));
                };
                v8 = v8 + 1;
            };
            if (0x1::vector::length<address>(&v6) > 0) {
                let v11 = RemoveOwnersEvent{owners_removed: v6};
                0x1::event::emit_event<RemoveOwnersEvent>(&mut v0.remove_owners_events, v11);
            };
        };
        if (0x1::option::is_some<u64>(&arg3)) {
            let v12 = 0x1::option::extract<u64>(&mut arg3);
            assert!(v12 > 0, 0x1::error::invalid_argument(11));
            let v13 = v0.num_signatures_required;
            if (v12 != v13) {
                v0.num_signatures_required = v12;
                let v14 = UpdateSignaturesRequiredEvent{
                    old_num_signatures_required : v13, 
                    new_num_signatures_required : v12,
                };
                0x1::event::emit_event<UpdateSignaturesRequiredEvent>(&mut v0.update_signature_required_events, v14);
            };
        };
        let v15 = 0x1::vector::length<address>(&v0.owners) >= v0.num_signatures_required;
        assert!(v15, 0x1::error::invalid_state(5));
    }
    
    entry fun update_signatures_required(arg0: &signer, arg1: u64) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), vector[], vector[], 0x1::option::some<u64>(arg1));
    }
    
    fun validate_multisig_transaction(arg0: &signer, arg1: address, arg2: vector<u8>) acquires MultisigAccount {
        assert_multisig_account_exists(arg1);
        let v0 = borrow_global<MultisigAccount>(arg1);
        assert_is_owner(arg0, v0);
        let v1 = v0.last_executed_sequence_number + 1;
        let v2 = 0x1::table::contains<u64, MultisigTransaction>(&v0.transactions, v1);
        assert!(v2, 0x1::error::invalid_argument(2006));
        let v3 = 0x1::table::borrow<u64, MultisigTransaction>(&v0.transactions, v1);
        let (v4, _) = num_approvals_and_rejections(&v0.owners, v3);
        assert!(v4 >= v0.num_signatures_required, 0x1::error::invalid_argument(2009));
        if (0x1::option::is_some<vector<u8>>(&v3.payload_hash)) {
            let v6 = 0x1::hash::sha3_256(arg2) == *0x1::option::borrow<vector<u8>>(&v3.payload_hash);
            assert!(v6, 0x1::error::invalid_argument(2008));
        };
    }
    
    fun validate_owners(arg0: &vector<address>, arg1: address) {
        let v0 = vector[];
        let v1 = 0;
        while (v1 < 0x1::vector::length<address>(arg0)) {
            let v2 = *0x1::vector::borrow<address>(arg0, v1);
            assert!(v2 != arg1, 0x1::error::invalid_argument(13));
            let (v3, _) = 0x1::vector::index_of<address>(&v0, &v2);
            assert!(!v3, 0x1::error::invalid_argument(1));
            0x1::vector::push_back<address>(&mut v0, v2);
            v1 = v1 + 1;
        };
    }
    
    public fun vote(arg0: address, arg1: u64, arg2: address) : (bool, bool) acquires MultisigAccount {
        let v0 = borrow_global_mut<MultisigAccount>(arg0);
        assert!(arg1 > 0 && arg1 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        let v1 = &0x1::table::borrow<u64, MultisigTransaction>(&v0.transactions, arg1).votes;
        let v2 = &arg2;
        let v3 = 0x1::simple_map::contains_key<address, bool>(v1, v2) && *0x1::simple_map::borrow<address, bool>(v1, &arg2);
        (v4, v3)
    }
    
    public entry fun vote_transanction(arg0: &signer, arg1: address, arg2: u64, arg3: bool) acquires MultisigAccount {
        assert_multisig_account_exists(arg1);
        let v0 = borrow_global_mut<MultisigAccount>(arg1);
        assert_is_owner(arg0, v0);
        let v1 = 0x1::table::contains<u64, MultisigTransaction>(&v0.transactions, arg2);
        assert!(v1, 0x1::error::not_found(2006));
        let v2 = &mut 0x1::table::borrow_mut<u64, MultisigTransaction>(&mut v0.transactions, arg2).votes;
        let v3 = 0x1::signer::address_of(arg0);
        if (0x1::simple_map::contains_key<address, bool>(v2, &v3)) {
            *0x1::simple_map::borrow_mut<address, bool>(v2, &v3) = arg3;
        } else {
            0x1::simple_map::add<address, bool>(v2, v3, arg3);
        };
        let v4 = VoteEvent{
            owner           : v3, 
            sequence_number : arg2, 
            approved        : arg3,
        };
        0x1::event::emit_event<VoteEvent>(&mut v0.vote_events, v4);
    }
    
    // decompiled from Move bytecode v6
}
