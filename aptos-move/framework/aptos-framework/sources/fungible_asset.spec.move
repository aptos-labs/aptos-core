spec aptos_framework::fungible_asset {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The metadata associated with the fungible asset is subject to precise size constraints.
    /// Criticality: Medium
    /// Implementation: The add_fungibility function has size limitations for the name, symbol, number of decimals,
    /// icon_uri, and project_uri field of the Metadata resource.
    /// Enforcement: Formally verified via [high-level-req-1](add_fungibility).
    ///
    /// No.: 2
    /// Requirement: Adding fungibility to an existing object should initialize the metadata and supply resources and store
    /// them under the metadata object address.
    /// Criticality: Low
    /// Implementation: The add_fungibility function initializes the Metadata and Supply resources and moves them under
    /// the metadata object.
    /// Enforcement: Audited that the Metadata and Supply resources are initialized properly.
    ///
    /// No.: 3
    /// Requirement: Generating mint, burn and transfer references can only be done at object creation time and if the
    /// object was added fungibility.
    /// Criticality: Low
    /// Implementation: The following functions generate the related references of the Metadata object: 1.
    /// generate_mint_ref 2. generate_burn_ref 3. generate_transfer_ref
    /// Enforcement: Audited that the Metadata object exists within the constructor ref.
    ///
    /// No.: 4
    /// Requirement: Only the owner of a store should be allowed to withdraw fungible assets from it.
    /// Criticality: High
    /// Implementation: The fungible_asset::withdraw function ensures that the signer owns the store by asserting that
    /// the object address matches the address of the signer.
    /// Enforcement: Audited that the address of the signer owns the object.
    ///
    /// No.: 5
    /// Requirement: The transfer, withdrawal and deposit operation should never change the current supply of the fungible
    /// asset.
    /// Criticality: High
    /// Implementation: The transfer function withdraws the fungible assets from the store and deposits them to the
    /// receiver. The withdraw function extracts the fungible asset from the fungible asset store. The deposit function
    /// adds the balance to the fungible asset store.
    /// Enforcement: Audited that the supply before and after the operation remains constant.
    ///
    /// No.: 6
    /// Requirement: The owner of the store should only be able to withdraw a certain amount if its store has sufficient
    /// balance and is not frozen, unless the withdrawal is performed with a reference, and afterwards the store balance
    /// should be decreased.
    /// Criticality: High
    /// Implementation: The withdraw function ensures that the store is not frozen before calling withdraw_internal
    /// which ensures that the withdrawing amount is greater than 0 and less than the total balance from the store.
    /// The withdraw_with_ref ensures that the reference's metadata matches the store metadata.
    /// Enforcement: Audited that it aborts if the withdrawing store is frozen. Audited that it aborts if the store doesn't have sufficient balance. Audited that the balance of the withdrawing store is reduced by amount.
    ///
    /// No.: 7
    /// Requirement: Only the same type of fungible assets should be deposited in a fungible asset store, if the store is
    /// not frozen, unless the deposit is performed with a reference, and afterwards the store balance should be
    /// increased.
    /// Criticality: High
    /// Implementation: The deposit function ensures that store is not frozen and proceeds to call the deposit_internal
    /// function which validates the store's metadata and the depositing asset's metadata followed by increasing the
    /// store balance by the given amount. The deposit_with_ref ensures that the reference's metadata matches the
    /// depositing asset's metadata.
    /// Enforcement: Audited that it aborts if the store is frozen. Audited that it aborts if the asset and asset store are different. Audited that the store's balance is increased by the deposited amount.
    ///
    /// No.: 8
    /// Requirement: An object should only be allowed to hold one store for fungible assets.
    /// Criticality: Medium
    /// Implementation: The create_store function initializes a new FungibleStore resource and moves it under the
    /// object address.
    /// Enforcement: Formally verified via [high-level-req-8](create_store).
    ///
    /// No.: 9
    /// Requirement: When a new store is created, the balance should be set by default to the value zero.
    /// Criticality: High
    /// Implementation: The create_store function initializes a new fungible asset store with zero balance and stores it
    /// under the given construtorRef object.
    /// Enforcement: Formally verified via [high-level-req-9](create_store).
    ///
    /// No.: 10
    /// Requirement: A store should only be deleted if its balance is zero.
    /// Criticality: Medium
    /// Implementation: The remove_store function validates the store's balance and removes the store under the object
    /// address.
    /// Enforcement: Formally verified via [high-level-req-10](remove_store).
    ///
    /// No.: 11
    /// Requirement: Minting and burning should alter the total supply value, and the store balances.
    /// Criticality: High
    /// Implementation: The mint process increases the total supply by the amount minted using the increase_supply
    /// function. The burn process withdraws the burn amount from the given store and decreases the total supply by the
    /// amount burned using the decrease_supply function.
    /// Enforcement: Audited the mint and burn functions that the supply was adjusted accordingly.
    ///
    /// No.: 12
    /// Requirement: It must not be possible to burn an amount of fungible assets larger than their current supply.
    /// Criticality: High
    /// Implementation: The burn process ensures that the store has enough balance to burn, by asserting that the
    /// supply.current >= amount inside the decrease_supply function.
    /// Enforcement: Audited that it aborts if the provided store doesn't have sufficient balance.
    ///
    /// No.: 13
    /// Requirement: Enabling or disabling store's frozen status should only be done with a valid transfer reference.
    /// Criticality: High
    /// Implementation: The set_frozen_flag function ensures that the TransferRef is provided via function argument and
    /// that the store's metadata matches the metadata from the reference. It then proceeds to update the frozen flag of
    /// the store.
    /// Enforcement: Audited that it aborts if the metadata doesn't match. Audited that the frozen flag is updated properly.
    ///
    /// No.: 14
    /// Requirement: Extracting a specific amount from the fungible asset should be possible only if the total amount that
    /// it holds is greater or equal to the provided amount.
    /// Criticality: High
    /// Implementation: The extract function validates that the fungible asset has enough balance to extract and then
    /// updates it by subtracting the extracted amount.
    /// Enforcement: Formally verified via [high-level-req-14](extract).
    ///
    /// No.: 15
    /// Requirement: Merging two fungible assets should only be possible if both share the same metadata.
    /// Criticality: Medium
    /// Implementation: The merge function validates the metadata of the src and dst asset.
    /// Enforcement: Formally verified via [high-level-req-15](merge).
    ///
    /// No.: 16
    /// Requirement: Post merging two fungible assets, the source asset should have the amount value equal to the sum of
    /// the two.
    /// Criticality: High
    /// Implementation: The merge function increases dst_fungible_asset.amount by src_fungible_asset.amount.
    /// Enforcement: Formally verified via [high-level-req-16](merge).
    ///
    /// No.: 17
    /// Requirement: Fungible assets with zero balance should be destroyed when the amount reaches value 0.
    /// Criticality: Medium
    /// Implementation: The destroy_zero ensures that the balance of the asset has the value 0 and destroy the asset.
    /// Enforcement: Formally verified via [high-level-req-17](destroy_zero).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
    }

    spec store_exists(store: address): bool {
        pragma opaque;
        aborts_if false;
        ensures result == exists<FungibleStore>(store);
    }

    spec name<T: key>(metadata: Object<T>): String {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).name;
    }

    spec symbol<T: key>(metadata: Object<T>): String {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).symbol;
    }

    spec decimals<T: key>(metadata: Object<T>): u8 {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).decimals;
    }

    spec icon_uri<T: key>(metadata: Object<T>): String {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).icon_uri;
    }

    spec project_uri<T: key>(metadata: Object<T>): String {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).project_uri;
    }

    spec metadata<T: key>(metadata: Object<T>): Metadata {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner);
    }

    spec store_metadata<T: key>(store: Object<T>): Object<Metadata> {
        pragma opaque;
        aborts_if !exists<FungibleStore>(store.inner);
        ensures result == global<FungibleStore>(store.inner).metadata;
    }

    spec is_untransferable<T: key>(metadata: Object<T>): bool {
        pragma opaque;
        aborts_if false;
        ensures result == exists<Untransferable>(metadata.inner);
    }

    spec is_frozen<T: key>(store: Object<T>): bool {
        pragma opaque;
        aborts_if false;
        ensures result == (exists<FungibleStore>(store.inner) && global<FungibleStore>(store.inner).frozen);
    }

    spec create_store<T: key>(constructor_ref: &ConstructorRef, metadata: Object<T>): Object<FungibleStore> {
        pragma aborts_if_is_partial;
        let addr = constructor_ref.self;
        /// [high-level-req-8]
        aborts_if exists<FungibleStore>(addr);
        ensures exists<FungibleStore>(addr);
        /// [high-level-req-9]
        ensures global<FungibleStore>(addr).balance == 0;
        ensures global<FungibleStore>(addr).frozen == false;
        ensures result.inner == addr;
    }

    spec remove_store(delete_ref: &DeleteRef) {
        pragma aborts_if_is_partial;
        let addr = delete_ref.self;
        aborts_if !exists<FungibleStore>(addr);
        /// [high-level-req-10]
        aborts_if global<FungibleStore>(addr).balance != 0;
        /// [high-level-req-10]
        aborts_if exists<ConcurrentFungibleBalance>(addr)
            && aggregator_v2::spec_get_value(global<ConcurrentFungibleBalance>(addr).balance) != 0;
        ensures !exists<FungibleStore>(addr);
    }

    spec extract(self: &mut FungibleAsset, amount: u64): FungibleAsset {
        /// [high-level-req-14]
        aborts_if self.amount < amount;
        ensures self.amount == old(self).amount - amount;
        ensures self.metadata == old(self).metadata;
        ensures result.amount == amount;
        ensures result.metadata == old(self).metadata;
    }

    spec merge(self: &mut FungibleAsset, src_fungible_asset: FungibleAsset) {
        /// [high-level-req-15]
        aborts_if src_fungible_asset.metadata != self.metadata;
        aborts_if self.amount + src_fungible_asset.amount > MAX_U64;
        /// [high-level-req-16]
        ensures self.amount == old(self).amount + src_fungible_asset.amount;
        ensures self.metadata == old(self).metadata;
    }

    spec destroy_zero(self: FungibleAsset) {
        /// [high-level-req-17]
        aborts_if self.amount != 0;
    }

    spec add_fungibility(
        constructor_ref: &ConstructorRef,
        maximum_supply: Option<u128>,
        name: String,
        symbol: String,
        decimals: u8,
        icon_uri: String,
        project_uri: String,
    ): Object<Metadata> {
        pragma aborts_if_is_partial;
        /// [high-level-req-1]
        aborts_if std::string::length(name) > MAX_NAME_LENGTH;
        aborts_if std::string::length(symbol) > MAX_SYMBOL_LENGTH;
        aborts_if decimals > MAX_DECIMALS;
        aborts_if std::string::length(icon_uri) > MAX_URI_LENGTH;
        aborts_if std::string::length(project_uri) > MAX_URI_LENGTH;
    }

    spec amount(self: &FungibleAsset): u64 {
        pragma opaque;
        aborts_if false;
        ensures result == self.amount;
    }

    spec asset_metadata(self: &FungibleAsset): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec metadata_from_asset(self: &FungibleAsset): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec mint_ref_metadata(self: &MintRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec transfer_ref_metadata(self: &TransferRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec burn_ref_metadata(self: &BurnRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec object_from_metadata_ref(self: &MutateMetadataRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec unchecked_withdraw {
        modifies global<FungibleStore>(store_addr);
        modifies global<ConcurrentFungibleBalance>(store_addr);
    }

    spec deposit {
        modifies global<FungibleStore>(object::object_address(store));
        modifies global<ConcurrentFungibleBalance>(object::object_address(store));
    }

    spec unchecked_deposit {
        modifies global<FungibleStore>(store_addr);
        modifies global<ConcurrentFungibleBalance>(store_addr);
    }

}
