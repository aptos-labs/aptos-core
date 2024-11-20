spec aptos_framework::native_bridge_store {
    spec initialize {
        let addr = signer::address_of(aptos_framework);
        ensures exists<Nonce>(addr);
        ensures exists<SmartTableWrapper<u64, OutboundTransfer<address, EthereumAddress>>>(addr);
        ensures exists<SmartTableWrapper<vector<u8>, u64>>(addr);
    }


    spec create_details<Initiator: store, Recipient: store>(initiator: Initiator, recipient: Recipient, amount: u64, nonce: u64)
    : OutboundTransfer<Initiator, Recipient> {
        aborts_if amount == 0;
        ensures result == OutboundTransfer<Initiator, Recipient> {
                addresses: AddressPair<Initiator, Recipient> {
                initiator,
                recipient
            },
            amount,
            nonce,
        };
    }

    spec schema AddAbortsIf<T> {
        nonce: u64;
        table: SmartTable<u64, T>;

        aborts_if nonce == 0;
        aborts_if smart_table::spec_contains(table, nonce);
        aborts_if !features::spec_is_enabled(features::NATIVE_BRIDGE);
    }

    spec add<Initiator: store, Recipient: store>(nonce: u64, details: OutboundTransfer<Initiator, Recipient>) {
        let table = global<SmartTableWrapper<u64, OutboundTransfer<Initiator, Recipient>>>(@aptos_framework).inner;
        include AddAbortsIf<OutboundTransfer<Initiator, Recipient>>;

        aborts_if !exists<SmartTableWrapper<u64, OutboundTransfer<Initiator, Recipient>>>(@aptos_framework);
        aborts_if smart_table::spec_contains(table, nonce);

        ensures smart_table::spec_contains(global<SmartTableWrapper<u64, OutboundTransfer<Initiator, Recipient>>>(@aptos_framework).inner, nonce);

        ensures smart_table::spec_len(global<SmartTableWrapper<u64, OutboundTransfer<Initiator, Recipient>>>(@aptos_framework).inner) ==
            old(smart_table::spec_len(global<SmartTableWrapper<u64, OutboundTransfer<Initiator, Recipient>>>(@aptos_framework).inner)) + 1;
    }

    spec schema OutboundTransferAbortsIf<Initiator, Recipient> {
        details: OutboundTransfer<Initiator, Recipient>;

        aborts_if !exists<CurrentTimeMicroseconds>(@aptos_framework);
    }

    spec complete_details<Initiator: store, Recipient: store + copy>(nonce: u64, details: &mut OutboundTransfer<Initiator, Recipient>) : (Recipient, u64) {
        include OutboundTransferAbortsIf<Initiator, Recipient>;
    }

    spec complete_transfer<Initiator: store, Recipient: copy + store>(bridge_transfer_id: vector<u8>, nonce: u64) : (Recipient, u64) {
        let table = global<SmartTableWrapper<u64, OutboundTransfer<Initiator, Recipient>>>(@aptos_framework).inner;
        aborts_if !features::spec_is_enabled(features::NATIVE_BRIDGE);
        aborts_if !exists<SmartTableWrapper<vector<u8>, OutboundTransfer<Initiator, Recipient>>>(@aptos_framework);
        aborts_if !smart_table::spec_contains(table, bridge_transfer_id);
        let details = smart_table::spec_get(table, bridge_transfer_id);
        include BridgetTransferDetailsAbortsIf<Initiator, Recipient>;
    }

    spec schema AbortBridgetTransferDetailsAbortsIf<Initiator, Recipient> {
        details: OutboundTransfer<Initiator, Recipient>;

        aborts_if details.state != PENDING_TRANSACTION;
        aborts_if !(timestamp::spec_now_seconds() > details.time_lock);
        aborts_if !exists<CurrentTimeMicroseconds>(@aptos_framework);
        ensures details.state == CANCELLED_TRANSACTION;
    }

    spec cancel_details<Initiator: store + copy, Recipient: store> (details: &mut OutboundTransfer<Initiator, Recipient>) : (Initiator, u64) {
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

spec aptos_framework::native_bridge_configuration {
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
