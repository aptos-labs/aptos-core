spec aptos_framework::aptos_coin {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The native token, APT, must be initialized during genesis.
    /// Criticality: Medium
    /// Implementation: The initialize function is only called once, during genesis.
    /// Enforcement: Formally verified via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: The APT coin may only be created exactly once.
    /// Criticality: Medium
    /// Implementation: The initialization function may only be called once.
    /// Enforcement: Enforced through the [https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move](coin)
    /// module, which has been audited.
    ///
    /// No.: 3
    /// Requirement: The abilities to mint Aptos tokens should be transferable, duplicatable, and destroyable.
    /// Criticality: High
    /// Implementation: The MintCapability struct has the copy and store abilities. This means that it can be duplicated
    /// and stored in different object wrappers (such as MintCapStore). This capability is tested against the
    /// destroy_mint_cap and claim_mint_capability functions.
    /// Enforcement: Verified via [high-level-req-3](initialize).

    /// No.: 4
    /// Requirement: Any type of operation on the APT coin should fail if the user has not registered for the coin.
    /// Criticality: Medium
    /// Implementation: Coin operations may succeed only on valid user coin registration.
    /// Enforcement: Enforced through the [https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move](coin)
    /// module, which has been audited.
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
    }

    spec initialize(aptos_framework: &signer): (BurnCapability<AptosCoin>, MintCapability<AptosCoin>) {

        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if !string::spec_internal_check_utf8(b"Aptos Coin");
        aborts_if !string::spec_internal_check_utf8(b"APT");
        aborts_if exists<MintCapStore>(addr);
        aborts_if exists<coin::CoinInfo<AptosCoin>>(addr);
        /// [high-level-req-1]
        ensures exists<MintCapStore>(addr);
        // property 3: The abilities to mint Aptos tokens should be transferable, duplicatable, and destroyable.
        /// [high-level-req-3]
        ensures global<MintCapStore>(addr).mint_cap ==  MintCapability<AptosCoin> {};
        ensures exists<coin::CoinInfo<AptosCoin>>(addr);
        ensures result_1 == BurnCapability<AptosCoin> {};
        ensures result_2 == MintCapability<AptosCoin> {};
    }

    spec destroy_mint_cap {
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if !exists<MintCapStore>(@aptos_framework);
    }

    /// Stores the mint capability and an empty delegation list at `core_resources`.
    spec configure_accounts_for_test(
        aptos_framework: &signer,
        core_resources: &signer,
        mint_cap: MintCapability<AptosCoin>,
    ) {
        let core_addr = signer::address_of(core_resources);
        aborts_if signer::address_of(aptos_framework) != @aptos_framework;
        aborts_if exists<MintCapStore>(core_addr);
        aborts_if exists<Delegations>(core_addr);
        ensures global<MintCapStore>(core_addr).mint_cap == mint_cap;
        ensures len(global<Delegations>(core_addr).inner) == 0;
    }

    /// Mints `amount` into the paired primary store of `dst_addr`.
    spec mint(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) {
        use aptos_std::type_info;
        use aptos_framework::fungible_asset;
        let paired = coin::spec_paired_metadata(type_info::type_of<AptosCoin>());
        let store_addr = coin::spec_paired_store_address(dst_addr, paired);
        aborts_if !exists<MintCapStore>(signer::address_of(account));
        aborts_if amount != 0 && !exists<coin::CoinInfo<AptosCoin>>(@aptos_framework);
        aborts_if !exists<coin::CoinConversionMap>(@aptos_framework);
        aborts_if coin::spec_has_paired_store(dst_addr, paired)
            && global<fungible_asset::FungibleStore>(store_addr).frozen;
        include coin::PairedDepositBalance<AptosCoin> { owner: dst_addr, amount };
    }

    /// Records a new delegation to `to`.
    spec delegate_mint_capability(account: signer, to: address) {
        let inner = global<Delegations>(@core_resources).inner;
        let post post_inner = global<Delegations>(@core_resources).inner;
        pragma aborts_if_is_partial = false;
        aborts_if signer::address_of(account) != @core_resources;
        aborts_if !exists<Delegations>(@core_resources);
        aborts_if exists i in 0..len(inner): inner[i].to == to;
        ensures len(post_inner) == len(inner) + 1;
        ensures post_inner[len(inner)].to == to;
    }

    /// Consumes the delegation to the signer and grants it a copy of the mint capability.
    spec claim_mint_capability(account: &signer) {
        let addr = signer::address_of(account);
        let inner = global<Delegations>(@core_resources).inner;
        let post post_inner = global<Delegations>(@core_resources).inner;
        aborts_if !exists<Delegations>(@core_resources);
        aborts_if !(exists i in 0..len(inner): inner[i].to == addr);
        aborts_if exists<MintCapStore>(addr);
        aborts_if !exists<MintCapStore>(@core_resources);
        ensures exists<MintCapStore>(addr);
        ensures len(post_inner) == len(inner) - 1;
        // The removed delegation is one to the signer, swapped with the last entry.
        ensures exists i in 0..len(inner): inner[i].to == addr
            && post_inner == update(inner, i, inner[len(inner) - 1])[0..len(inner) - 1];
    }

    /// Returns the index of a delegation to `addr`, if any.
    spec find_delegation(addr: address): Option<u64> {
        let inner = global<Delegations>(@core_resources).inner;
        aborts_if !exists<Delegations>(@core_resources);
        ensures option::is_some(result) ==> option::borrow(result) < len(inner)
            && inner[option::borrow(result)].to == addr;
    }

    spec schema ExistsAptosCoin {
        requires exists<coin::CoinInfo<AptosCoin>>(@aptos_framework);
    }

}
