spec aptos_framework::bridge_store {
    spec initialize {
        let addr = signer::address_of(aptos_framework);

        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if exists<Nonce>(addr);
        aborts_if exists<BridgeTransferStore>(addr);
        ensures exists<Nonce>(addr);
        ensures exists<BridgeTransferStore>(addr);
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

    spec create_details<Initiator, Recipient>(initiator: Initiator, recipient: Recipient, amount: u64, hash_lock: vector<u8>, time_lock: u64)
    : BridgeTransferDetails<AddressPair<Initiator, Recipient>> {
        include TimeLockAbortsIf;
        aborts_if amount == 0;
        aborts_if len(hash_lock) != 32;
        ensures result == BridgeTransferDetails<AddressPair<Initiator, Recipient>> {
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
        table: smart_table::SmartTable<vector<u8>, T>;

        aborts_if !exists<BridgeTransferStore>(@aptos_framework);
        aborts_if len(bridge_transfer_id) != 32;
        aborts_if smart_table::spec_contains(table, bridge_transfer_id);
    }

    spec add_initiator(bridge_transfer_id: vector<u8>, details: BridgeTransferDetails<AddressPair<address, EthereumAddress>>) {
        let table = global<BridgeTransferStore>(@aptos_framework).initiators;
        include AddAbortsIf<BridgeTransferDetails<AddressPair<address, EthereumAddress>>>;

        ensures smart_table::spec_contains(global<BridgeTransferStore>(@aptos_framework).initiators, bridge_transfer_id);
    }

    spec add_counterparty {
        let table = global<BridgeTransferStore>(@aptos_framework).counterparties;
        include AddAbortsIf<BridgeTransferDetails<AddressPair<EthereumAddress, address>>>;

        ensures smart_table::spec_contains(borrow_global<BridgeTransferStore>(@aptos_framework).counterparties, bridge_transfer_id);
    }

    spec schema HashLockAbortsIf {
        hash_lock: vector<u8>;
        aborts_if len(hash_lock) != 32;
    }

    spec schema BridgetTransferDetailsAbortsIf<T> {
        hash_lock: vector<u8>;
        details: BridgeTransferDetails<T>;
        include HashLockAbortsIf;

        aborts_if details.state != PENDING_TRANSACTION;
        aborts_if details.hash_lock != hash_lock;
    }

    spec schema BridgeTransferStoreAbortsIf<T> {
        bridge_transfer_id: vector<u8>;
        table: smart_table::SmartTable<vector<u8>, T>;

        aborts_if !exists<BridgeTransferStore>(@aptos_framework);
        aborts_if !smart_table::spec_contains(table, bridge_transfer_id);
    }

    spec complete_details<_, Recipient: copy>(hash_lock: vector<u8>, details: &mut BridgeTransferDetails<AddressPair<_, Recipient>>) : (Recipient, u64) {
        include BridgetTransferDetailsAbortsIf<AddressPair<_, Recipient>>;
    }

    spec complete_counterparty {
        let table = global<BridgeTransferStore>(@aptos_framework).counterparties;
        include BridgeTransferStoreAbortsIf<BridgeTransferDetails<AddressPair<EthereumAddress, address>>>;
        let details = smart_table::spec_get(table, bridge_transfer_id);
        include BridgetTransferDetailsAbortsIf<AddressPair<EthereumAddress, address>>;
    }

    spec complete_initiator {
        let table = global<BridgeTransferStore>(@aptos_framework).initiators;
        include BridgeTransferStoreAbortsIf<BridgeTransferDetails<AddressPair<address, EthereumAddress>>>;
        let details = smart_table::spec_get(table, bridge_transfer_id);
        include BridgetTransferDetailsAbortsIf<AddressPair<address, EthereumAddress>>;
    }

    spec schema AbortBridgetTransferDetailsAbortsIf<T> {
        details: BridgeTransferDetails<T>;

        aborts_if details.state != PENDING_TRANSACTION;
        aborts_if timestamp::spec_now_seconds() <= details.time_lock;
        aborts_if !exists<CurrentTimeMicroseconds>(@aptos_framework);
        ensures details.state == CANCELLED_TRANSACTION;
    }

    spec cancel_details<Initiator: copy, _> (details: &mut BridgeTransferDetails<AddressPair<Initiator, _>>) : (Initiator, u64) {
        include AbortBridgetTransferDetailsAbortsIf<AddressPair<Initiator, _>>;
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

spec aptos_framework::bridge_configuration {
    spec initialize(aptos_framework: &signer) {
        aborts_if !system_addresses::is_aptos_framework_address(address_of(aptos_framework));
        aborts_if exists<BridgeConfig>(address_of(aptos_framework));

        ensures global<BridgeConfig>(address_of(aptos_framework)).bridge_operator == address_of(aptos_framework);
    }

    spec update_bridge_operator(aptos_framework: &signer, new_operator: address) {
        aborts_if !system_addresses::is_aptos_framework_address(address_of(aptos_framework));
        aborts_if !exists<BridgeConfig>(address_of(aptos_framework));
        aborts_if global<BridgeConfig>(address_of(aptos_framework)).bridge_operator == new_operator;

        ensures global<BridgeConfig>(address_of(aptos_framework)).bridge_operator == new_operator;
    }
}