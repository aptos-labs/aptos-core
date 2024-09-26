spec aptos_framework::atomic_bridge_store {
    spec initialize {
        let addr = signer::address_of(aptos_framework);
        ensures exists<Nonce>(addr);
        ensures exists<SmartTableWrapper<vector<u8>, BridgeTransferDetails<address, EthereumAddress>>>(addr);
        ensures exists<SmartTableWrapper<vector<u8>, BridgeTransferDetails<EthereumAddress, address>>>(addr);
    }

    spec schema TimeLockAbortsIf {
        time_lock: u64;
        aborts_if time_lock < MIN_TIME_LOCK;
        aborts_if !exists<CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if time_lock > MAX_U64 - timestamp::spec_now_seconds();
    }

    spec create_time_lock {
        include TimeLockAbortsIf;
        ensures result == timestamp::spec_now_seconds() + time_lock;
        /// If the sum of `now()` and `lock` does not overflow, the result is the sum of `now()` and `lock`.
        ensures (timestamp::spec_now_seconds() + time_lock <= 0xFFFFFFFFFFFFFFFF) ==> result == timestamp::spec_now_seconds() + time_lock;
    }

    spec create_details<Initiator: store, Recipient: store>(initiator: Initiator, recipient: Recipient, amount: u64, hash_lock: vector<u8>, time_lock: u64)
    : BridgeTransferDetails<Initiator, Recipient> {
        include TimeLockAbortsIf;
        aborts_if amount == 0;
        aborts_if len(hash_lock) != 32;
        ensures result == BridgeTransferDetails<Initiator, Recipient> {
                addresses: AddressPair<Initiator, Recipient> {
                initiator,
                recipient
            },
            amount,
            hash_lock,
            time_lock: timestamp::spec_now_seconds() + time_lock,
            state: PENDING_TRANSACTION,
        };
    }

    spec schema AddAbortsIf<T> {
        bridge_transfer_id: vector<u8>;
        table: SmartTable<vector<u8>, T>;

        aborts_if len(bridge_transfer_id) != 32;
        aborts_if smart_table::spec_contains(table, bridge_transfer_id);
    }

    spec add<Initiator: store, Recipient: store>(bridge_transfer_id: vector<u8>, details: BridgeTransferDetails<Initiator, Recipient>) {
        let table = global<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework).inner;
        include AddAbortsIf<BridgeTransferDetails<Initiator, Recipient>>;

        aborts_if !exists<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework);
        aborts_if smart_table::spec_contains(table, bridge_transfer_id);

        ensures smart_table::spec_contains(global<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework).inner, bridge_transfer_id);

        ensures smart_table::spec_len(global<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework).inner) ==
            old(smart_table::spec_len(global<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework).inner)) + 1;
    }

    spec schema HashLockAbortsIf {
        hash_lock: vector<u8>;
        aborts_if len(hash_lock) != 32;
    }

    spec schema BridgetTransferDetailsAbortsIf<Initiator, Recipient> {
        hash_lock: vector<u8>;
        details: BridgeTransferDetails<Initiator, Recipient>;
        include HashLockAbortsIf;

        aborts_if details.state != PENDING_TRANSACTION;
        aborts_if details.hash_lock != hash_lock;
    }

    spec complete_details<Initiator: store, Recipient: store + copy>(hash_lock: vector<u8>, details: &mut BridgeTransferDetails<Initiator, Recipient>) : (Recipient, u64) {
        include BridgetTransferDetailsAbortsIf<Initiator, Recipient>;
    }

    spec complete_transfer<Initiator: store, Recipient: copy + store>(bridge_transfer_id: vector<u8>, hash_lock: vector<u8>) : (Recipient, u64) {
        let table = global<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework).inner;
        aborts_if !exists<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework);
        aborts_if !smart_table::spec_contains(table, bridge_transfer_id);
        let details = smart_table::spec_get(table, bridge_transfer_id);
        include BridgetTransferDetailsAbortsIf<Initiator, Recipient>;
    }

    spec schema AbortBridgetTransferDetailsAbortsIf<Initiator, Recipient> {
        details: BridgeTransferDetails<Initiator, Recipient>;

        aborts_if details.state != PENDING_TRANSACTION;
        aborts_if timestamp::spec_now_seconds() <= details.time_lock;
        aborts_if !exists<CurrentTimeMicroseconds>(@aptos_framework);
        ensures details.state == CANCELLED_TRANSACTION;
    }

    spec cancel_details<Initiator: store + copy, Recipient: store> (details: &mut BridgeTransferDetails<Initiator, Recipient>) : (Initiator, u64) {
        include AbortBridgetTransferDetailsAbortsIf<Initiator, Recipient>;
    }

    spec create_hashlock {
        aborts_if len(pre_image) == 0;
    }

    spec complete {
        requires details.state == PENDING_TRANSACTION;
        ensures details.state == COMPLETED_TRANSACTION;
    }

    spec cancel {
        requires details.state == PENDING_TRANSACTION;
        ensures details.state == CANCELLED_TRANSACTION;
    }
}

spec aptos_framework::atomic_bridge_configuration {
    spec initialize(aptos_framework: &signer) {
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        aborts_if exists<BridgeConfig>(signer::address_of(aptos_framework));

        ensures global<BridgeConfig>(signer::address_of(aptos_framework)).bridge_operator == signer::address_of(aptos_framework);
    }

    spec update_bridge_operator(aptos_framework: &signer, new_operator: address) {
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        aborts_if !exists<BridgeConfig>(signer::address_of(aptos_framework));
        aborts_if global<BridgeConfig>(signer::address_of(aptos_framework)).bridge_operator == new_operator;

        ensures global<BridgeConfig>(signer::address_of(aptos_framework)).bridge_operator == new_operator;
    }
}