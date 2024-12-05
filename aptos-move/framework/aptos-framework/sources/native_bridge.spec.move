spec aptos_framework::native_bridge {

    //use std::features;
    //use aptos_framework::coin;

    // spec initialize(aptos_framework: &signer) {
    //     aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
    //     aborts_if exists<Nonce>(signer::address_of(aptos_framework));
    //     aborts_if exists<BridgeEvents>(signer::address_of(aptos_framework));

    //     ensures exists<Nonce>(signer::address_of(aptos_framework));
    //     ensures global<Nonce>(signer::address_of(aptos_framework)).value == 1;

    //     ensures exists<BridgeEvents>(signer::address_of(aptos_framework));
    //     ensures
    //         global<BridgeEvents>(signer::address_of(aptos_framework))
    //             .bridge_transfer_initiated_events.counter == 0;
    //     ensures
    //         global<BridgeEvents>(signer::address_of(aptos_framework))
    //             .bridge_transfer_completed_events.counter == 0;
    // }

    // spec increment_and_get_nonce {
    //     aborts_if !exists<Nonce>(@aptos_framework);

    //     ensures global<Nonce>(@aptos_framework).value == old(global<Nonce>(@aptos_framework).value) + 1;
    //     ensures result == global<Nonce>(@aptos_framework).value;
    // }

    // spec initiate_bridge_transfer(
    //     initiator: &signer,
    //     recipient: vector<u8>,
    //     amount: u64
    // ) {
    //     aborts_if amount == 0;
    //     aborts_if !exists<Nonce>(@aptos_framework);
    //     aborts_if !exists<BridgeEvents>(@aptos_framework);

    //     ensures global<Nonce>(@aptos_framework).value == old(global<Nonce>(@aptos_framework).value) + 1;

    //     ensures
    //         global<BridgeEvents>(@aptos_framework).bridge_transfer_initiated_events.counter ==
    //         old(
    //             global<BridgeEvents>(@aptos_framework).bridge_transfer_initiated_events.counter
    //         ) + 1;
    // }

    // spec complete_bridge_transfer(
    //     caller: &signer,
    //     bridge_transfer_id: vector<u8>,
    //     initiator: vector<u8>,
    //     recipient: address,
    //     amount: u64,
    //     nonce: u64
    // ) {
    //     // Abort if the caller is not a relayer
    //     aborts_if !exists<native_bridge_configuration::BridgeConfig>(@aptos_framework);
    //     aborts_if global<native_bridge_configuration::BridgeConfig>(@aptos_framework).bridge_relayer != signer::address_of(caller);

    //     // Abort if the bridge transfer ID is already associated with an incoming nonce
    //     aborts_if native_bridge_store::is_incoming_nonce_set(bridge_transfer_id);

    //     // Abort if the `BridgeEvents` resource does not exist
    //     aborts_if !exists<BridgeEvents>(@aptos_framework);

    //     // Ensure the bridge transfer ID is associated with an incoming nonce after execution
    //     ensures native_bridge_store::is_incoming_nonce_set(bridge_transfer_id);

    //     // Ensure the event counter is incremented by 1
    //     ensures
    //         global<BridgeEvents>(@aptos_framework).bridge_transfer_completed_events.counter ==
    //         old(
    //             global<BridgeEvents>(@aptos_framework).bridge_transfer_completed_events.counter
    //         ) + 1;
    // }
}

spec aptos_framework::native_bridge_core {

    spec initialize(aptos_framework: &signer) {
        pragma aborts_if_is_partial = true;

        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        // aborts_if exists<AptosCoinBurnCapability>(@aptos_framework);
        // aborts_if exists<AptosCoinMintCapability>(@aptos_framework);

        // ensures exists<AptosCoinBurnCapability>(@aptos_framework);
        // ensures exists<AptosCoinMintCapability>(@aptos_framework);
    }
    // spec store_aptos_coin_burn_cap(aptos_framework: &signer, burn_cap: BurnCapability<AptosCoin>) {
    //     aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
    //     aborts_if exists<AptosCoinBurnCapability>(@aptos_framework);

    //     ensures exists<AptosCoinBurnCapability>(@aptos_framework);
    // }

    // spec store_aptos_coin_mint_cap(aptos_framework: &signer, mint_cap: MintCapability<AptosCoin>) {
    //     aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
    //     aborts_if exists<AptosCoinMintCapability>(@aptos_framework);

    //     ensures exists<AptosCoinMintCapability>(@aptos_framework);
    // }

    // spec mint(recipient: address, amount: u64) {
    //     aborts_if !exists<AptosCoinMintCapability>(@aptos_framework);
    //     aborts_if amount == 0;

    //     ensures coin::balance<AptosCoin>(recipient) == old(coin::balance<AptosCoin>(recipient)) + amount;
    // }

    // spec burn(from: address, amount: u64) {
    //     aborts_if !exists<AptosCoinBurnCapability>(@aptos_framework);
    //     aborts_if coin::balance<AptosCoin>(from) < amount;

    //     ensures coin::balance<AptosCoin>(from) == old(coin::balance<AptosCoin>(from)) - amount;
    // }
}

spec aptos_framework::native_bridge_store {

    // spec initialize(aptos_framework: &signer) {
    //     aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));

    //     ensures exists<SmartTableWrapper<u64, OutboundTransfer>>(@aptos_framework);
    //     ensures exists<SmartTableWrapper<vector<u8>, u64>>(@aptos_framework);
    // }

    // spec is_incoming_nonce_set(bridge_transfer_id: vector<u8>): bool {
    //     ensures result == exists<SmartTableWrapper<vector<u8>, u64>>(@aptos_framework)
    //         && smart_table::spec_contains(
    //             global<SmartTableWrapper<vector<u8>, u64>>(@aptos_framework).inner,
    //             bridge_transfer_id
    //         );
    // }

    // spec create_details(
    //     initiator: address,
    //     recipient: EthereumAddress,
    //     amount: u64,
    //     nonce: u64
    // ): OutboundTransfer {
    //     aborts_if amount == 0;

    //     ensures result.bridge_transfer_id == bridge_transfer_id(
    //         initiator,
    //         recipient,
    //         amount,
    //         nonce
    //     );
    //     ensures result.initiator == initiator;
    //     ensures result.recipient == recipient;
    //     ensures result.amount == amount;
    // }

    // spec add(nonce: u64, details: OutboundTransfer) {
    //     aborts_if !exists<SmartTableWrapper<u64, OutboundTransfer>>(@aptos_framework);
    //     aborts_if smart_table::spec_contains(
    //         global<SmartTableWrapper<u64, OutboundTransfer>>(@aptos_framework).inner,
    //         nonce
    //     );

    //     ensures smart_table::spec_contains(
    //         global<SmartTableWrapper<u64, OutboundTransfer>>(@aptos_framework).inner,
    //         nonce
    //     );
    //     ensures smart_table::spec_len(
    //         global<SmartTableWrapper<u64, OutboundTransfer>>(@aptos_framework).inner
    //     ) == old(smart_table::spec_len(
    //         global<SmartTableWrapper<u64, OutboundTransfer>>(@aptos_framework).inner
    //     )) + 1;
    // }

    // spec set_bridge_transfer_id_to_inbound_nonce(
    //     bridge_transfer_id: vector<u8>,
    //     inbound_nonce: u64
    // ) {
    //     aborts_if !exists<SmartTableWrapper<vector<u8>, u64>>(@aptos_framework);

    //     ensures smart_table::spec_contains(
    //         global<SmartTableWrapper<vector<u8>, u64>>(@aptos_framework).inner,
    //         bridge_transfer_id
    //     );
    // }
    /*
    spec bridge_transfer_id(
        initiator: address,
        recipient: EthereumAddress,
        amount: u64,
        nonce: u64
    ): vector<u8> {
        let combined_bytes = vec_empty<u8>();
        combined_bytes = vector::append(combined_bytes, bcs::to_bytes(&initiator));
        combined_bytes = vector::append(combined_bytes, bcs::to_bytes(&recipient));
        combined_bytes = vector::append(combined_bytes, bcs::to_bytes(&amount));
        combined_bytes = vector::append(combined_bytes, bcs::to_bytes(&nonce));

        ensures result == aptos_std::aptos_hash::keccak256(combined_bytes);
    }
    */
}

// spec aptos_framework::native_bridge_configuration {

//     spec initialize(aptos_framework: &signer) {
//         aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
//         aborts_if exists<BridgeConfig>(signer::address_of(aptos_framework));

//         ensures exists<BridgeConfig>(signer::address_of(aptos_framework));
//         ensures global<BridgeConfig>(signer::address_of(aptos_framework)).bridge_relayer == signer::address_of(aptos_framework);
//     }

//     spec update_bridge_relayer(aptos_framework: &signer, new_relayer: address) {
//         aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
//         aborts_if !exists<BridgeConfig>(signer::address_of(aptos_framework));
//         aborts_if global<BridgeConfig>(signer::address_of(aptos_framework)).bridge_relayer == new_relayer;

//         ensures global<BridgeConfig>(signer::address_of(aptos_framework)).bridge_relayer == new_relayer;
//     }

//     spec bridge_relayer(): address {
//         aborts_if !exists<BridgeConfig>(@aptos_framework);

//         ensures result == global<BridgeConfig>(@aptos_framework).bridge_relayer;
//     }

//     spec assert_is_caller_relayer(caller: &signer) {
//         aborts_if !exists<BridgeConfig>(@aptos_framework);
//         aborts_if global<BridgeConfig>(@aptos_framework).bridge_relayer != signer::address_of(caller);
//     }
// }
