spec aptos_framework::transaction_fee {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Given the blockchain is in an operating state, it guarantees that the Aptos framework signer may burn
    /// Aptos coins.
    /// Criticality: Critical
    /// Implementation: The AptosCoinCapabilities structure is defined in this module and it stores burn capability to
    /// burn the gas fees.
    /// Enforcement: Formally Verified via [high-level-req-1](module).
    ///
    /// No.: 2
    /// Requirement: The initialization function may only be called once.
    /// Criticality: Medium
    /// Implementation: The initialize_fee_collection_and_distribution function ensures CollectedFeesPerBlock does not
    /// already exist.
    /// Enforcement: The fee collection mechanism no longer exists; initialize_fee_collection_and_distribution always
    /// aborts.
    ///
    /// No.: 3
    /// Requirement: Only the admin address is authorized to call the initialization function.
    /// Criticality: Critical
    /// Implementation: The initialize_fee_collection_and_distribution function ensures only the Aptos framework address
    /// calls it.
    /// Enforcement: The fee collection mechanism no longer exists; initialize_fee_collection_and_distribution always
    /// aborts.
    ///
    /// No.: 4
    /// Requirement: The percentage of the burnt collected fee is always a value from 0 to 100.
    /// Criticality: Medium
    /// Implementation: During the initialization of CollectedFeesPerBlock in
    /// Initialize_fee_collection_and_distribution, and while upgrading burn percentage, it asserts that burn_percentage
    /// is within the specified limits.
    /// Enforcement: Formally verified via [high-level-req-4](CollectedFeesPerBlock).
    ///
    /// No.: 5
    /// Requirement: Prior to upgrading the burn percentage, it must process all the fees collected up to that point.
    /// Criticality: Critical
    /// Implementation: The upgrade_burn_percentage function ensures process_collected_fees function is called before
    /// updating the burn percentage.
    /// Enforcement: The fee collection mechanism no longer exists; upgrade_burn_percentage always aborts.
    ///
    /// No.: 6
    /// Requirement: The presence of the resource, indicating collected fees per block under the Aptos framework account,
    /// is a prerequisite for the successful execution of the following functionalities: Upgrading burn percentage.
    /// Registering a block proposer. Processing collected fees.
    /// Criticality: Low
    /// Implementation: The functions: upgrade_burn_percentage, register_proposer_for_fee_collection, and
    /// process_collected_fees all ensure that the CollectedFeesPerBlock resource exists under aptos_framework by
    /// calling the is_fees_collection_enabled method, which returns a boolean value confirming if the resource exists
    /// or not.
    /// Enforcement: The fee collection mechanism no longer exists; CollectedFeesPerBlock is never created and
    /// upgrade_burn_percentage always aborts.
    /// </high-level-req>
    ///
    spec module {
        use aptos_framework::chain_status;

        pragma aborts_if_is_strict;
        // property 1: Given the blockchain is in an operating state, it guarantees that the Aptos framework signer may burn Aptos coins.
        /// [high-level-req-1]
        invariant [suspendable] chain_status::is_operating() ==> exists<AptosCoinCapabilities>(@aptos_framework) || exists<AptosFABurnCapabilities>(@aptos_framework);
    }

    spec CollectedFeesPerBlock {
        // property 4: The percentage of the burnt collected fee is always a value from 0 to 100.
        /// [high-level-req-4]
        invariant burn_percentage <= 100;
    }

    /// Deprecated; always aborts.
    spec initialize_fee_collection_and_distribution(_aptos_framework: &signer, _burn_percentage: u8) {
        aborts_if true;
    }

    /// Deprecated; always aborts.
    spec upgrade_burn_percentage(_aptos_framework: &signer, _new_burn_percentage: u8) {
        aborts_if true;
    }

    /// Deprecated; always aborts.
    spec convert_to_aptos_fa_burn_ref(_aptos_framework: &signer) {
        aborts_if true;
    }

    /// Burns `fee` from the APT primary store of `account` with the stored burn ref.
    spec burn_fee(account: address, fee: u64) {
        use aptos_framework::aptos_account;
        aborts_if !exists<AptosFABurnCapabilities>(@aptos_framework);
        include aptos_account::BurnFromAptStore {
            ref: global<AptosFABurnCapabilities>(@aptos_framework).burn_ref,
            amount: fee
        };
    }

    /// Mints `refund` into the primary store of `account`, with the FA mint ref if enabled, else with the coin
    /// mint capability.
    spec mint_and_refund(account: address, refund: u64) {
        use std::features;
        use std::option;
        use aptos_std::type_info;
        use aptos_framework::aptos_account;
        use aptos_framework::fungible_asset;
        // Partial: the abort conditions of minting and of creating the pairing or the store are not specified.
        pragma aborts_if_is_partial;
        let fa_path = features::spec_is_enabled(features::GAS_REFUND_FA_MINT)
            && exists<AptosFAMintCapabilities>(@aptos_framework);
        let apt_store = aptos_account::spec_apt_store_address(account);
        let paired = coin::spec_paired_metadata(type_info::type_of<AptosCoin>());
        let coin_store = coin::spec_paired_store_address(account, paired);
        let apt_store_exists = exists<fungible_asset::FungibleStore>(apt_store);
        let coin_store_exists = coin::spec_has_paired_store(account, paired);
        aborts_if !fa_path && !exists<AptosCoinMintCapability>(@aptos_framework);
        // A missing store is created, except for a zero refund on the FA path.
        ensures fa_path && (apt_store_exists || refund != 0) ==> exists<fungible_asset::FungibleStore>(apt_store)
            && fungible_asset::spec_balance(apt_store)
                == (if (apt_store_exists) { old(fungible_asset::spec_balance(apt_store)) } else { 0 }) + refund;
        ensures !fa_path && option::is_some(paired) ==> exists<fungible_asset::FungibleStore>(coin_store)
            && fungible_asset::spec_balance(coin_store)
                == (if (coin_store_exists) { old(fungible_asset::spec_balance(coin_store)) } else { 0 }) + refund;
    }

    /// Ensure caller is admin.
    /// Takes the paired burn ref of APT and stores it.
    spec store_aptos_coin_burn_cap(aptos_framework: &signer, burn_cap: BurnCapability<AptosCoin>) {
        use std::option;
        use std::signer;
        use aptos_std::type_info;
        let addr = signer::address_of(aptos_framework);
        let metadata = option::borrow(coin::spec_paired_metadata(type_info::type_of<AptosCoin>()));
        let refs = global<coin::PairedFungibleAssetRefs>(metadata.inner);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        include coin::PairedRefsAbortsIf<AptosCoin>;
        aborts_if option::is_none(refs.burn_ref_opt);
        aborts_if exists<AptosFABurnCapabilities>(addr);
        modifies global<AptosFABurnCapabilities>(addr);
        modifies global<coin::PairedFungibleAssetRefs>(metadata.inner);
        ensures exists<AptosFABurnCapabilities>(addr);
        ensures global<AptosFABurnCapabilities>(addr).burn_ref == option::borrow(refs.burn_ref_opt);
    }

    /// Ensure caller is admin.
    /// Aborts if `AptosFAMintCapabilities` or `AptosCoinMintCapability` already exists.
    spec store_aptos_coin_mint_cap(aptos_framework: &signer, mint_cap: MintCapability<AptosCoin>) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        include PairedMintRefAbortsIf;
        aborts_if exists<AptosFAMintCapabilities>(addr);
        aborts_if exists<AptosCoinMintCapability>(addr);
        modifies global<AptosFAMintCapabilities>(addr);
        modifies global<AptosCoinMintCapability>(addr);
        ensures exists<AptosFAMintCapabilities>(addr);
        ensures exists<AptosCoinMintCapability>(addr);
    }

    /// APT is paired and its mint ref has not been taken.
    spec schema PairedMintRefAbortsIf {
        use std::option;
        use aptos_std::type_info;
        let metadata = option::borrow(coin::spec_paired_metadata(type_info::type_of<AptosCoin>()));
        include coin::PairedRefsAbortsIf<AptosCoin>;
        aborts_if option::is_none(global<coin::PairedFungibleAssetRefs>(metadata.inner).mint_ref_opt);
    }

    /// Ensure caller is admin.
    spec convert_to_aptos_fa_mint_ref(aptos_framework: &signer) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if !exists<AptosCoinMintCapability>(addr);
        include PairedMintRefAbortsIf;
        aborts_if exists<AptosFAMintCapabilities>(addr);
        ensures exists<AptosFAMintCapabilities>(addr);
    }

    /// Historical. Aborts.
    spec initialize_storage_refund(_: &signer) {
        aborts_if true;
    }

    /// Never aborts.
    spec emit_fee_statement {}
}
