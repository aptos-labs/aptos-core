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
        use 0x1::object;
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).name;
        ensures [inferred] Metadata[object::object_address<T>(metadata)].name
            == Metadata[metadata.inner].name ==>
            result == Metadata[object::object_address<T>(metadata)].name;
        aborts_if [inferred]!exists<Metadata>(object::object_address<T>(metadata));
    }

    spec symbol<T: key>(metadata: Object<T>): String {
        use 0x1::object;
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).symbol;
        ensures [inferred] Metadata[object::object_address<T>(metadata)].symbol
            == Metadata[metadata.inner].symbol ==>
            result == Metadata[object::object_address<T>(metadata)].symbol;
        aborts_if [inferred]!exists<Metadata>(object::object_address<T>(metadata));
    }

    spec decimals<T: key>(metadata: Object<T>): u8 {
        use 0x1::object;
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).decimals;
        ensures [inferred] Metadata[object::object_address<T>(metadata)].decimals
            == Metadata[metadata.inner].decimals ==>
            result == Metadata[object::object_address<T>(metadata)].decimals;
        aborts_if [inferred]!exists<Metadata>(object::object_address<T>(metadata));
    }

    spec icon_uri<T: key>(metadata: Object<T>): String {
        use 0x1::object;
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).icon_uri;
        ensures [inferred] Metadata[object::object_address<T>(metadata)].icon_uri
            == Metadata[metadata.inner].icon_uri ==>
            result == Metadata[object::object_address<T>(metadata)].icon_uri;
        aborts_if [inferred]!exists<Metadata>(object::object_address<T>(metadata));
    }

    spec project_uri<T: key>(metadata: Object<T>): String {
        use 0x1::object;
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).project_uri;
        ensures [inferred] Metadata[object::object_address<T>(metadata)].project_uri
            == Metadata[metadata.inner].project_uri ==>
            result == Metadata[object::object_address<T>(metadata)].project_uri;
        aborts_if [inferred]!exists<Metadata>(object::object_address<T>(metadata));
    }

    spec metadata<T: key>(metadata: Object<T>): Metadata {
        use 0x1::object;
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner);
        ensures [inferred] Metadata[object::object_address<T>(metadata)]
            == Metadata[metadata.inner] ==>
            result == Metadata[object::object_address<T>(metadata)];
        aborts_if [inferred]!exists<Metadata>(object::object_address<T>(metadata));
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
        use 0x1::object;
        pragma opaque;
        aborts_if false;
        ensures result
            == (
                exists<FungibleStore>(store.inner)
                    && global<FungibleStore>(store.inner).frozen
            );
        ensures [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && FungibleStore[object::object_address<T>(store)].frozen
                == (
                    exists<FungibleStore>(store.inner)
                        && FungibleStore[store.inner].frozen
                ) ==>
            result == FungibleStore[object::object_address<T>(store)].frozen;
        ensures [inferred]!exists<FungibleStore>(object::object_address<T>(store))
            && false
                == (
                    exists<FungibleStore>(store.inner)
                        && FungibleStore[store.inner].frozen
                ) ==> result == false;
        aborts_if [inferred] false;
    }

    spec create_store<T: key>(
        constructor_ref: &ConstructorRef, metadata: Object<T>
    ): Object<FungibleStore> {
        use 0x1::object;
        use 0x1::features;
        pragma opaque = true;
        // The signer produced by the constructor and the final conversion both
        // address the object identified by this constructor reference.
        let addr = constructor_ref.self;
        // `create_store` publishes the store and can initialize the object's
        // transfer guard and concurrent-balance sidecar.  Keep the complete
        // conditional footprint explicit so opaque callers can frame it.
        modifies object::ObjectCore[addr];
        modifies FungibleStore[addr];
        modifies ConcurrentFungibleBalance[addr];
        modifies object::Untransferable[addr];
        /// [high-level-req-8]
        aborts_if exists<FungibleStore>(addr);
        // The final object conversion checks the constructor address has an
        // ObjectCore. This is normally established by object creation, but it
        // is an explicit pre-state abort condition of this public function.
        aborts_if !exists<object::ObjectCore>(addr);
        // `metadata.convert()` requires an ObjectCore and Metadata at the
        // supplied metadata object address.
        aborts_if !exists<object::ObjectCore>(metadata.inner);
        aborts_if !exists<Metadata>(metadata.inner);
        aborts_if features::spec_is_enabled(68)
            && exists<ConcurrentFungibleBalance>(addr);
        aborts_if exists<Untransferable>(metadata.inner)
            && exists<object::Untransferable>(addr);
        ensures exists<FungibleStore>(addr);
        /// [high-level-req-9]
        ensures global<FungibleStore>(addr).balance == 0;
        ensures global<FungibleStore>(addr).frozen == false;
        ensures result.inner == addr;
    }

    spec remove_store(delete_ref: &DeleteRef) {
        use 0x1::object;
        pragma aborts_if_is_partial;
        let addr = delete_ref.self;
        aborts_if !exists<FungibleStore>(addr);
        /// [high-level-req-10]
        aborts_if global<FungibleStore>(addr).balance != 0;
        /// [high-level-req-10]
        aborts_if exists<ConcurrentFungibleBalance>(addr)
            && aggregator_v2::spec_get_value(
                global<ConcurrentFungibleBalance>(addr).balance
            ) != 0;
        ensures !exists<FungibleStore>(addr);
        pragma opaque = true;
        modifies FungibleAssetEvents[
            object::object_address<FungibleStore>(
                object::Object<FungibleStore> { inner: delete_ref.self }
            )
        ];
        aborts_if [inferred] FungibleStore[
            object::object_address<FungibleStore>(
                object::Object<FungibleStore> { inner: delete_ref.self }
            )
        ].balance != 0;
        aborts_if [inferred]!exists<FungibleStore>(
            object::object_address<FungibleStore>(
                object::Object<FungibleStore> { inner: delete_ref.self }
            )
        );
        aborts_if [inferred] aborts_of<object::object_from_delete_ref<FungibleStore>> (
            delete_ref
        );
    }

    spec extract(self: &mut FungibleAsset, amount: u64): FungibleAsset {
        /// [high-level-req-14]
        aborts_if self.amount < amount;
        ensures self.amount == old(self).amount - amount;
        ensures self.metadata == old(self).metadata;
        ensures result.amount == amount;
        ensures result.metadata == old(self).metadata;
        pragma opaque = true;
        ensures [inferred] old(self).amount >= amount
            && (self.amount == old(self).amount - amount
                && self.metadata == old(self).metadata) ==>
            result == FungibleAsset { metadata: self.metadata, amount: amount };
        ensures [inferred] old(self).amount >= amount ==>
            self == update_field(old(self), amount, old(self).amount - amount);
        ensures [inferred] old(self).amount < amount ==> self == old(self);
        aborts_if [inferred] self.amount >= amount && self.amount - amount < 0;
        aborts_if [inferred] self.amount < amount;
    }

    spec merge(
        self: &mut FungibleAsset, src_fungible_asset: FungibleAsset
    ) {
        /// [high-level-req-15]
        aborts_if src_fungible_asset.metadata != self.metadata;
        aborts_if self.amount + src_fungible_asset.amount > MAX_U64;
        /// [high-level-req-16]
        ensures self.amount == old(self).amount + src_fungible_asset.amount;
        ensures self.metadata == old(self).metadata;
        pragma opaque = true;
        ensures [inferred] src_fungible_asset.metadata == old(self).metadata ==>
            self
                == update_field(
                    old(self), amount, old(self).amount + src_fungible_asset.amount
                );
        ensures [inferred] src_fungible_asset.metadata != old(self).metadata ==>
            self == old(self);
        aborts_if [inferred] src_fungible_asset.metadata == self.metadata
            && self.amount + src_fungible_asset.amount > MAX_U64;
        aborts_if [inferred] src_fungible_asset.metadata != self.metadata;
    }

    spec destroy_zero(self: FungibleAsset) {
        /// [high-level-req-17]
        aborts_if self.amount != 0;
        pragma opaque = true;
        aborts_if [inferred] self.amount != 0;
    }

    spec add_fungibility(
        constructor_ref: &ConstructorRef,
        maximum_supply: Option<u128>,
        name: String,
        symbol: String,
        decimals: u8,
        icon_uri: String,
        project_uri: String
    ): Object<Metadata> {
        use 0x1::features;
        use 0x1::object;
        use 0x1::option;
        use 0x1::aggregator_v2;
        pragma opaque = true, aborts_if_is_partial = false;
        // Trusted boundary: body verification reaches a Boogie type-generation
        // error for String's byte vector (Vec bv8 versus Vec int), independent of the
        // selected postcondition shape. The straight-line source contract is complete.
        pragma verify = false;
        let metadata_address = constructor_ref.self;
        modifies Metadata[metadata_address];
        modifies Supply[metadata_address];
        modifies ConcurrentSupply[metadata_address];
        /// [high-level-req-1]
        aborts_if constructor_ref.can_delete;
        aborts_if std::string::length(name) > MAX_NAME_LENGTH;
        aborts_if std::string::length(symbol) > MAX_SYMBOL_LENGTH;
        aborts_if decimals > MAX_DECIMALS;
        aborts_if std::string::length(icon_uri) > MAX_URI_LENGTH;
        aborts_if std::string::length(project_uri) > MAX_URI_LENGTH;
        aborts_if exists<Metadata>(metadata_address);
        aborts_if !exists<object::ObjectCore>(metadata_address);
        aborts_if features::concurrent_fungible_assets_enabled()
            && exists<ConcurrentSupply>(metadata_address);
        aborts_if !features::concurrent_fungible_assets_enabled()
            && exists<Supply>(metadata_address);
        ensures result == Object<Metadata> { inner: metadata_address };
        ensures Metadata[metadata_address].name == name;
        ensures Metadata[metadata_address].symbol == symbol;
        ensures Metadata[metadata_address].decimals == decimals;
        ensures Metadata[metadata_address].icon_uri == icon_uri;
        ensures Metadata[metadata_address].project_uri == project_uri;
        ensures features::concurrent_fungible_assets_enabled() ==>
            exists<ConcurrentSupply>(metadata_address);
        ensures features::concurrent_fungible_assets_enabled() ==>
            ConcurrentSupply[metadata_address].current
                == if (option::is_none(maximum_supply)) {
                    result_of<aggregator_v2::create_unbounded_aggregator<u128>> ()
                } else {
                    result_of<aggregator_v2::create_aggregator<u128>> (
                        option::spec_borrow(maximum_supply)
                    )
                };
        ensures !features::concurrent_fungible_assets_enabled() ==>
            Supply[metadata_address] == Supply { current: 0, maximum: maximum_supply };
    }

    spec amount(self: &FungibleAsset): u64 {
        pragma opaque;
        aborts_if false;
        ensures result == self.amount;
        ensures [inferred] result == self.amount;
        aborts_if [inferred] false;
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
        ensures [inferred] result == self.metadata;
        aborts_if [inferred] false;
    }

    spec mint_ref_metadata(self: &MintRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
        ensures [inferred] result == self.metadata;
        aborts_if [inferred] false;
    }

    spec transfer_ref_metadata(self: &TransferRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
        ensures [inferred] result == self.metadata;
        aborts_if [inferred] false;
    }

    spec burn_ref_metadata(self: &BurnRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
        ensures [inferred] result == self.metadata;
        aborts_if [inferred] false;
    }

    spec object_from_metadata_ref(self: &MutateMetadataRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
        ensures [inferred] result == self.metadata;
        aborts_if [inferred] false;
    }

    spec unchecked_withdraw {
        use 0x1::event;
        use 0x1::aggregator_v2;
        modifies global<FungibleStore>(store_addr);
        modifies global<ConcurrentFungibleBalance>(store_addr);
        pragma opaque = true;
        ensures exists<FungibleStore>(store_addr)
            == old(exists<FungibleStore>(store_addr));
        ensures exists<ConcurrentFungibleBalance>(store_addr)
            == old(exists<ConcurrentFungibleBalance>(store_addr));
        ensures old(exists<FungibleStore>(store_addr)) ==>
            result
                == FungibleAsset {
                    metadata: old(FungibleStore[store_addr]).metadata,
                    amount: amount
                };
        ensures old(exists<FungibleStore>(store_addr)) ==>
            FungibleStore[store_addr].metadata
                == old(FungibleStore[store_addr]).metadata
                && FungibleStore[store_addr].frozen
                    == old(FungibleStore[store_addr]).frozen;
        ensures old(exists<FungibleStore>(store_addr))
            && old(FungibleStore[store_addr]).balance == 0
            && old(exists<ConcurrentFungibleBalance>(store_addr)) ==>
            FungibleStore[store_addr] == old(FungibleStore[store_addr]);
        ensures old(exists<FungibleStore>(store_addr))
            && !(
                old(FungibleStore[store_addr]).balance == 0
                    && old(exists<ConcurrentFungibleBalance>(store_addr))
            ) ==>
            FungibleStore[store_addr].balance
                == old(FungibleStore[store_addr]).balance - amount;
        ensures old(exists<FungibleStore>(store_addr))
            && old(exists<ConcurrentFungibleBalance>(store_addr))
            && old(FungibleStore[store_addr]).balance == 0 ==>
            aggregator_v2::spec_get_value(ConcurrentFungibleBalance[store_addr].balance)
                == aggregator_v2::spec_get_value(
                    old(ConcurrentFungibleBalance[store_addr].balance)
                ) - amount;
        ensures old(exists<FungibleStore>(store_addr))
            && old(exists<ConcurrentFungibleBalance>(store_addr))
            && old(FungibleStore[store_addr]).balance == 0 ==>
            aggregator_v2::spec_get_max_value(
                ConcurrentFungibleBalance[store_addr].balance
            ) == aggregator_v2::spec_get_max_value(
                old(ConcurrentFungibleBalance[store_addr].balance)
            );
        ensures old(exists<FungibleStore>(store_addr))
            && old(exists<ConcurrentFungibleBalance>(store_addr))
            && old(FungibleStore[store_addr]).balance != 0 ==>
            ConcurrentFungibleBalance[store_addr]
                == old(ConcurrentFungibleBalance[store_addr]);
        ensures amount != 0 ==>
            ensures_of<event::emit<Withdraw>> (Withdraw { store: store_addr, amount: amount });
        aborts_if !exists<FungibleStore>(store_addr);
        aborts_if amount != 0
            && exists<FungibleStore>(store_addr)
            && FungibleStore[store_addr].balance == 0
            && exists<ConcurrentFungibleBalance>(store_addr)
            && aggregator_v2::spec_get_value(
                ConcurrentFungibleBalance[store_addr].balance
            ) < amount;
        aborts_if amount != 0
            && exists<FungibleStore>(store_addr)
            && !(
                FungibleStore[store_addr].balance == 0
                    && exists<ConcurrentFungibleBalance>(store_addr)
            )
            && FungibleStore[store_addr].balance < amount;
        aborts_if amount != 0
            && exists<FungibleStore>(store_addr)
            && (
                (
                    FungibleStore[store_addr].balance == 0
                        && exists<ConcurrentFungibleBalance>(store_addr)
                        && aggregator_v2::spec_get_value(
                            ConcurrentFungibleBalance[store_addr].balance
                        ) >= amount
                )
                    || (
                        !(
                            FungibleStore[store_addr].balance == 0
                                && exists<ConcurrentFungibleBalance>(store_addr)
                        )
                            && FungibleStore[store_addr].balance >= amount
                    )
            )
            && aborts_of<event::emit<Withdraw>> (Withdraw { store: store_addr, amount: amount });
    }

    spec deposit {
        use 0x1::object;
        modifies global<FungibleStore>(object::object_address(store));
        modifies global<ConcurrentFungibleBalance>(object::object_address(store));
        pragma opaque = true;
        ensures [inferred] S1.. |~(
            ensures_of<unchecked_deposit>(object::object_address<T>(store), fa)
        );
        ensures [inferred]..S1 |~(ensures_of<deposit_sanity_check<T>> (store, true));
        aborts_if [inferred] S1 |~(
            aborts_of<unchecked_deposit>(object::object_address<T>(store), fa)
        );
        aborts_if [inferred] aborts_of<deposit_sanity_check<T>> (store, true);
    }

    spec unchecked_deposit {
        use 0x1::error;
        use 0x1::event;
        use 0x1::aggregator_v2;
        modifies global<FungibleStore>(store_addr);
        modifies global<ConcurrentFungibleBalance>(store_addr);
        pragma opaque = true;
        ensures [inferred] exists<FungibleStore>(store_addr)
            && (fa.metadata == FungibleStore[store_addr].metadata
                && fa.amount != 0) ==>
            ensures_of<event::emit<Deposit>> (Deposit { store: store_addr, amount: fa.amount });
        ensures [inferred] exists<FungibleStore>(store_addr)
            && (
                fa.metadata == FungibleStore[store_addr].metadata
                    && (fa.amount != 0
                        && FungibleStore[store_addr].balance == 0)
            ) ==>
            ConcurrentFungibleBalance[store_addr].balance
                == ConcurrentFungibleBalance[store_addr].balance;
        ensures [inferred] exists<FungibleStore>(store_addr)
            && (
                fa.metadata == FungibleStore[store_addr].metadata
                    && (fa.amount != 0
                        && FungibleStore[store_addr].balance == 0)
            ) ==>
            ensures_of<aggregator_v2::add<u64>> (
                ConcurrentFungibleBalance[store_addr].balance, fa.amount
            );
        ensures [inferred] exists<FungibleStore>(store_addr)
            && (
                fa.metadata == FungibleStore[store_addr].metadata
                    && (fa.amount != 0
                        && FungibleStore[store_addr].balance != 0)
            ) ==>
            update<FungibleStore>(
                store_addr,
                update_field(
                    FungibleStore[store_addr],
                    balance,
                    FungibleStore[store_addr].balance + fa.amount
                )
            );
        aborts_if [inferred]!exists<FungibleStore>(store_addr);
        aborts_if [inferred] exists<FungibleStore>(store_addr)
            && fa.metadata != FungibleStore[store_addr].metadata;
        aborts_if [inferred] exists<FungibleStore>(store_addr)
            && (
                fa.metadata != FungibleStore[store_addr].metadata
                    && aborts_of<error::invalid_argument>(11)
            );
        aborts_if [inferred] exists<FungibleStore>(store_addr)
            && (
                fa.metadata == FungibleStore[store_addr].metadata
                    && (
                        fa.amount != 0
                            && aborts_of<event::emit<Deposit>> (
                                Deposit { store: store_addr, amount: fa.amount }
                            )
                    )
            );
        aborts_if [inferred] exists<FungibleStore>(store_addr)
            && (
                fa.metadata == FungibleStore[store_addr].metadata
                    && (
                        fa.amount != 0
                            && (
                                FungibleStore[store_addr].balance != 0
                                    && FungibleStore[store_addr].balance + fa.amount
                                        > MAX_U64
                            )
                    )
            );
    }

    spec burn(
        self: &0x1::fungible_asset::BurnRef, fa: 0x1::fungible_asset::FungibleAsset
    ) {
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        modifies ConcurrentSupply[object::object_address<Metadata>(self.metadata)];
        modifies Supply[object::object_address<Metadata>(self.metadata)];
        ensures self.metadata == fa.metadata ==>
            exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
                == old(
                    exists<ConcurrentSupply>(
                        object::object_address<Metadata>(self.metadata)
                    )
                );
        ensures self.metadata == fa.metadata ==>
            exists<Supply>(object::object_address<Metadata>(self.metadata))
                == old(
                    exists<Supply>(object::object_address<Metadata>(self.metadata))
                );
        ensures self.metadata == fa.metadata
            && old(
                exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            ) ==>
            aggregator_v2::spec_get_value(
                ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
            ) == aggregator_v2::spec_get_value(
                old(
                    ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
                )
            ) - (fa.amount as u128);
        ensures self.metadata == fa.metadata
            && old(
                exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            ) ==>
            aggregator_v2::spec_get_max_value(
                ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
            ) == aggregator_v2::spec_get_max_value(
                old(
                    ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
                )
            );
        ensures self.metadata == fa.metadata
            && old(
                exists<Supply>(object::object_address<Metadata>(self.metadata))
            )
            && !old(
                exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            ) ==>
            Supply[object::object_address<Metadata>(self.metadata)].maximum
                == old(
                    Supply[object::object_address<Metadata>(self.metadata)].maximum
                )
                && Supply[object::object_address<Metadata>(self.metadata)].current
                    == old(
                        Supply[object::object_address<Metadata>(self.metadata)].current
                    ) - (fa.amount as u128);
        ensures self.metadata == fa.metadata
            && old(
                exists<Supply>(object::object_address<Metadata>(self.metadata))
            )
            && old(
                exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            ) ==>
            Supply[object::object_address<Metadata>(self.metadata)]
                == old(
                    Supply[object::object_address<Metadata>(self.metadata)]
                );
        aborts_if self.metadata != fa.metadata;
        aborts_if self.metadata == fa.metadata && aborts_of<burn_internal>(fa);
    }

    spec generate_transfer_ref(
        constructor_ref: &0x1::object::ConstructorRef
    ): 0x1::fungible_asset::TransferRef {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] result
            == TransferRef {
                metadata: object::Object<Metadata> { inner: constructor_ref.self }
            };
        aborts_if [inferred] aborts_of<object::object_from_constructor_ref<Metadata>> (
            constructor_ref
        );
    }

    spec set_untransferable(
        constructor_ref: &0x1::object::ConstructorRef
    ) {
        use 0x1::signer;
        use 0x1::object;
        pragma opaque = true;
        modifies Untransferable[
            signer::address_of(result_of<object::generate_signer>(constructor_ref))
        ];
        ensures [inferred] old(
            exists<Metadata>(object::address_from_constructor_ref(constructor_ref))
        ) ==>
            {
                let a =
                    signer::address_of(
                        ..S1 |~ result_of<object::generate_signer>(constructor_ref)
                    );
                S1.. |~ publish<Untransferable>(a, Untransferable {})
            };
        aborts_if [inferred] exists<Metadata>(
            object::address_from_constructor_ref(constructor_ref)
        ) && (
            S1 |~ exists<Untransferable>(
                signer::address_of(
                    ..S1 |~ result_of<object::generate_signer>(constructor_ref)
                )
            )
        );
        aborts_if [inferred]!exists<Metadata>(
            object::address_from_constructor_ref(constructor_ref)
        );
    }

    spec transfer<T: key>(
        sender: &signer,
        from: 0x1::object::Object<T>,
        to: 0x1::object::Object<T>,
        amount: u64
    ) {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a = ..S1 |~ result_of<withdraw<T>> (sender, from, amount);
            S1.. |~ ensures_of<deposit<T>> (to, a)
        });
        aborts_if [inferred]({
            let a = ..S1 |~ result_of<withdraw<T>> (sender, from, amount);
            S1 |~ aborts_of<deposit<T>> (to, a)
        });
    }

    spec transfer_with_ref<T: key>(
        self: &0x1::fungible_asset::TransferRef,
        from: 0x1::object::Object<T>,
        to: 0x1::object::Object<T>,
        amount: u64
    ) {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a = ..S1 |~ result_of<withdraw_with_ref<T>> (self, from, amount);
            S1.. |~ ensures_of<deposit_with_ref<T>> (self, to, a)
        });
        aborts_if [inferred]({
            let a = ..S1 |~ result_of<withdraw_with_ref<T>> (self, from, amount);
            S1 |~ aborts_of<deposit_with_ref<T>> (self, to, a)
        });
    }

    spec address_burn_from_for_gas(
        self: &0x1::fungible_asset::BurnRef, store_addr: address, amount: u64
    ) {
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        modifies FungibleStore[store_addr];
        modifies ConcurrentFungibleBalance[store_addr];
        modifies Supply[object::object_address<Metadata>(self.metadata)];
        modifies ConcurrentSupply[object::object_address<Metadata>(self.metadata)];
        ensures exists<FungibleStore>(store_addr)
            == old(exists<FungibleStore>(store_addr));
        ensures exists<ConcurrentFungibleBalance>(store_addr)
            == old(exists<ConcurrentFungibleBalance>(store_addr));
        ensures old(exists<FungibleStore>(store_addr)) ==>
            self.metadata == old(FungibleStore[store_addr]).metadata;
        ensures old(exists<FungibleStore>(store_addr)) ==>
            FungibleStore[store_addr].metadata
                == old(FungibleStore[store_addr]).metadata
                && FungibleStore[store_addr].frozen
                    == old(FungibleStore[store_addr]).frozen;
        ensures old(exists<FungibleStore>(store_addr))
            && old(FungibleStore[store_addr]).balance == 0
            && old(exists<ConcurrentFungibleBalance>(store_addr)) ==>
            FungibleStore[store_addr] == old(FungibleStore[store_addr]);
        ensures old(exists<FungibleStore>(store_addr))
            && !(
                old(FungibleStore[store_addr]).balance == 0
                    && old(exists<ConcurrentFungibleBalance>(store_addr))
            ) ==>
            FungibleStore[store_addr].balance
                == old(FungibleStore[store_addr]).balance - amount;
        ensures old(exists<FungibleStore>(store_addr))
            && old(exists<ConcurrentFungibleBalance>(store_addr))
            && old(FungibleStore[store_addr]).balance == 0 ==>
            aggregator_v2::spec_get_value(ConcurrentFungibleBalance[store_addr].balance)
                == aggregator_v2::spec_get_value(
                    old(ConcurrentFungibleBalance[store_addr].balance)
                ) - amount;
        ensures old(exists<FungibleStore>(store_addr))
            && old(exists<ConcurrentFungibleBalance>(store_addr))
            && old(FungibleStore[store_addr]).balance == 0 ==>
            aggregator_v2::spec_get_max_value(
                ConcurrentFungibleBalance[store_addr].balance
            ) == aggregator_v2::spec_get_max_value(
                old(ConcurrentFungibleBalance[store_addr].balance)
            );
        ensures old(exists<FungibleStore>(store_addr))
            && old(exists<ConcurrentFungibleBalance>(store_addr))
            && old(FungibleStore[store_addr]).balance != 0 ==>
            ConcurrentFungibleBalance[store_addr]
                == old(ConcurrentFungibleBalance[store_addr]);
        ensures old(exists<FungibleStore>(store_addr)) ==>
            exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
                == old(
                    exists<ConcurrentSupply>(
                        object::object_address<Metadata>(self.metadata)
                    )
                );
        ensures old(exists<FungibleStore>(store_addr)) ==>
            exists<Supply>(object::object_address<Metadata>(self.metadata))
                == old(
                    exists<Supply>(object::object_address<Metadata>(self.metadata))
                );
        ensures old(exists<FungibleStore>(store_addr))
            && old(
                exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            ) ==>
            aggregator_v2::spec_get_value(
                ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
            ) == aggregator_v2::spec_get_value(
                old(
                    ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
                )
            ) - (amount as u128);
        ensures old(exists<FungibleStore>(store_addr))
            && old(
                exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            ) ==>
            aggregator_v2::spec_get_max_value(
                ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
            ) == aggregator_v2::spec_get_max_value(
                old(
                    ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
                )
            );
        ensures old(exists<FungibleStore>(store_addr))
            && old(
                exists<Supply>(object::object_address<Metadata>(self.metadata))
            )
            && !old(
                exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            ) ==>
            Supply[object::object_address<Metadata>(self.metadata)].maximum
                == old(
                    Supply[object::object_address<Metadata>(self.metadata)].maximum
                )
                && Supply[object::object_address<Metadata>(self.metadata)].current
                    == old(
                        Supply[object::object_address<Metadata>(self.metadata)].current
                    ) - (amount as u128);
        ensures old(exists<FungibleStore>(store_addr))
            && old(
                exists<Supply>(object::object_address<Metadata>(self.metadata))
            )
            && old(
                exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            ) ==>
            Supply[object::object_address<Metadata>(self.metadata)]
                == old(
                    Supply[object::object_address<Metadata>(self.metadata)]
                );
        aborts_if !exists<FungibleStore>(store_addr);
        aborts_if amount != 0
            && exists<FungibleStore>(store_addr)
            && FungibleStore[store_addr].balance == 0
            && exists<ConcurrentFungibleBalance>(store_addr)
            && aggregator_v2::spec_get_value(
                ConcurrentFungibleBalance[store_addr].balance
            ) < amount;
        aborts_if amount != 0
            && exists<FungibleStore>(store_addr)
            && !(
                FungibleStore[store_addr].balance == 0
                    && exists<ConcurrentFungibleBalance>(store_addr)
            )
            && FungibleStore[store_addr].balance < amount;
        aborts_if exists<FungibleStore>(store_addr)
            && (
                (amount == 0)
                    || (
                        FungibleStore[store_addr].balance == 0
                            && exists<ConcurrentFungibleBalance>(store_addr)
                            && aggregator_v2::spec_get_value(
                                ConcurrentFungibleBalance[store_addr].balance
                            ) >= amount
                    )
                    || (
                        !(
                            FungibleStore[store_addr].balance == 0
                                && exists<ConcurrentFungibleBalance>(store_addr)
                        )
                            && FungibleStore[store_addr].balance >= amount
                    )
            )
            && self.metadata != FungibleStore[store_addr].metadata;
        aborts_if exists<FungibleStore>(store_addr)
            && self.metadata == FungibleStore[store_addr].metadata
            && (
                (amount == 0)
                    || (
                        FungibleStore[store_addr].balance == 0
                            && exists<ConcurrentFungibleBalance>(store_addr)
                            && aggregator_v2::spec_get_value(
                                ConcurrentFungibleBalance[store_addr].balance
                            ) >= amount
                    )
                    || (
                        !(
                            FungibleStore[store_addr].balance == 0
                                && exists<ConcurrentFungibleBalance>(store_addr)
                        )
                            && FungibleStore[store_addr].balance >= amount
                    )
            )
            && aborts_of<decrease_supply<Metadata>> (self.metadata, amount);
    }

    spec balance<T: key>(store: 0x1::object::Object<T>): u64 {
        use 0x1::error;
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        ensures [inferred]!exists<FungibleStore>(object::object_address<T>(store)) ==> result
            == 0;
        ensures [inferred = sathard]({
            let a =
                S1 |~ exists<ConcurrentFungibleBalance>(object::object_address<T>(store));
            let b =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            let c = {
                let d =
                    S1 |~ global<ConcurrentFungibleBalance>(
                        object::object_address<T>(store)
                    );
                S1.. |~ result_of<aggregator_v2::read<u64>> (d.balance)
            };
            exists<FungibleStore>(object::object_address<T>(store))
                && (
                    !b
                        && (FungibleStore[object::object_address<T>(store)].balance
                            == 0
                            && a)
                ) ==> result == c
        });
        ensures [inferred]({
            let a =
                S1 |~ exists<ConcurrentFungibleBalance>(object::object_address<T>(store));
            let b =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            exists<FungibleStore>(object::object_address<T>(store))
                && (
                    !b
                        && (FungibleStore[object::object_address<T>(store)].balance
                            == 0
                            && !a)
                ) ==>
                result == FungibleStore[object::object_address<T>(store)].balance
        });
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            exists<FungibleStore>(object::object_address<T>(store))
                && (!a
                    && FungibleStore[object::object_address<T>(store)].balance != 0) ==>
                result == FungibleStore[object::object_address<T>(store)].balance
        });
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                )
            );
        aborts_if [inferred]({
            let a =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            exists<FungibleStore>(object::object_address<T>(store))
                && (a
                    && aborts_of<error::invalid_argument>(28))
        });
    }

    spec balance_snapshot<T: key>(
        store: 0x1::object::Object<T>
    ): 0x1::aggregator_v2::AggregatorSnapshot<u64> {
        use 0x1::error;
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        ensures [inferred]!exists<FungibleStore>(object::object_address<T>(store)) ==>
            result == result_of<aggregator_v2::create_snapshot<u64>> (0);
        ensures [inferred]({
            let a =
                S1 |~ exists<ConcurrentFungibleBalance>(object::object_address<T>(store));
            let b =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            let c = {
                let d =
                    S1 |~ global<ConcurrentFungibleBalance>(
                        object::object_address<T>(store)
                    );
                S1.. |~ result_of<aggregator_v2::snapshot<u64>> (d.balance)
            };
            exists<FungibleStore>(object::object_address<T>(store))
                && (
                    !b
                        && (FungibleStore[object::object_address<T>(store)].balance
                            == 0
                            && a)
                ) ==> result == c
        });
        ensures [inferred]({
            let a =
                S1 |~ exists<ConcurrentFungibleBalance>(object::object_address<T>(store));
            let b =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            let c =
                S1.. |~ result_of<aggregator_v2::create_snapshot<u64>> (
                    FungibleStore[object::object_address<T>(store)].balance
                );
            exists<FungibleStore>(object::object_address<T>(store))
                && (
                    !b
                        && (FungibleStore[object::object_address<T>(store)].balance
                            == 0
                            && !a)
                ) ==> result == c
        });
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            let b =
                S1.. |~ result_of<aggregator_v2::create_snapshot<u64>> (
                    FungibleStore[object::object_address<T>(store)].balance
                );
            exists<FungibleStore>(object::object_address<T>(store))
                && (!a
                    && FungibleStore[object::object_address<T>(store)].balance != 0) ==> result
                == b
        });
        aborts_if [inferred]!exists<FungibleStore>(object::object_address<T>(store))
            && aborts_of<aggregator_v2::create_snapshot<u64>> (0);
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                )
            );
        aborts_if [inferred]({
            let a =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            exists<FungibleStore>(object::object_address<T>(store))
                && (a
                    && aborts_of<error::invalid_argument>(28))
        });
        aborts_if [inferred]({
            let a =
                S1 |~ exists<ConcurrentFungibleBalance>(object::object_address<T>(store));
            let b =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            let c = {
                let d =
                    S1 |~ global<ConcurrentFungibleBalance>(
                        object::object_address<T>(store)
                    );
                S1 |~ aborts_of<aggregator_v2::snapshot<u64>> (d.balance)
            };
            exists<FungibleStore>(object::object_address<T>(store))
                && (
                    !b
                        && (
                            FungibleStore[object::object_address<T>(store)].balance
                                == 0
                                && (a && c)
                        )
                )
        });
        aborts_if [inferred]({
            let a =
                S1 |~ exists<ConcurrentFungibleBalance>(object::object_address<T>(store));
            let b =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            let c =
                S1 |~ aborts_of<aggregator_v2::create_snapshot<u64>> (
                    FungibleStore[object::object_address<T>(store)].balance
                );
            exists<FungibleStore>(object::object_address<T>(store))
                && (
                    !b
                        && (
                            FungibleStore[object::object_address<T>(store)].balance
                                == 0
                                && (!a && c)
                        )
                )
        });
        aborts_if [inferred]({
            let a =
                ..S1 |~ result_of<has_balance_dispatch_function>(
                    FungibleStore[object::object_address<T>(store)].metadata
                );
            let b =
                S1 |~ aborts_of<aggregator_v2::create_snapshot<u64>> (
                    FungibleStore[object::object_address<T>(store)].balance
                );
            exists<FungibleStore>(object::object_address<T>(store))
                && (
                    !a
                        && (FungibleStore[object::object_address<T>(store)].balance
                            != 0
                            && b)
                )
        });
    }

    spec balance_with_ref<T: key>(
        self: &0x1::fungible_asset::RawBalanceRef, store: 0x1::object::Object<T>
    ): u64 {
        use 0x1::error;
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        ensures [inferred]!exists<FungibleStore>(object::object_address<T>(store)) ==> result
            == 0;
        ensures [inferred = sathard] exists<FungibleStore>(
            object::object_address<T>(store)
        )
            && (
                self.metadata
                    == FungibleStore[object::object_address<T>(store)].metadata
                    && FungibleStore[object::object_address<T>(store)].balance == 0
            ) ==>
            result
                == result_of<aggregator_v2::read<u64>> (
                    ConcurrentFungibleBalance[object::object_address<T>(store)].balance
                );
        ensures [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                self.metadata
                    == FungibleStore[object::object_address<T>(store)].metadata
                    && FungibleStore[object::object_address<T>(store)].balance != 0
            ) ==>
            result == FungibleStore[object::object_address<T>(store)].balance;
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && self.metadata
                != FungibleStore[object::object_address<T>(store)].metadata;
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                self.metadata
                    != FungibleStore[object::object_address<T>(store)].metadata
                    && aborts_of<error::invalid_argument>(34)
            );
    }

    spec burn_from<T: key>(
        self: &0x1::fungible_asset::BurnRef, store: 0x1::object::Object<T>, amount: u64
    ) {
        use 0x1::object;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<unchecked_withdraw>(
                    object::object_address<T>(store), amount
                );
            S1.. |~ ensures_of<burn>(self, a)
        });
    }

    spec burn_internal(self: 0x1::fungible_asset::FungibleAsset): u64 {
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        modifies ConcurrentSupply[object::object_address<Metadata>(self.metadata)];
        modifies Supply[object::object_address<Metadata>(self.metadata)];
        ensures result == self.amount;
        ensures exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            == old(
            exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
        );
        ensures exists<Supply>(object::object_address<Metadata>(self.metadata))
            == old(
                exists<Supply>(object::object_address<Metadata>(self.metadata))
            );
        ensures old(
            exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
        ) ==>
            aggregator_v2::spec_get_value(
                ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
            ) == aggregator_v2::spec_get_value(
                old(
                    ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
                )
            ) - (self.amount as u128);
        ensures old(
            exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
        ) ==>
            aggregator_v2::spec_get_max_value(
                ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
            ) == aggregator_v2::spec_get_max_value(
                old(
                    ConcurrentSupply[object::object_address<Metadata>(self.metadata)].current
                )
            );
        ensures old(
            exists<Supply>(object::object_address<Metadata>(self.metadata))
        )
            && !old(
                exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
            ) ==>
            Supply[object::object_address<Metadata>(self.metadata)].maximum
                == old(
                    Supply[object::object_address<Metadata>(self.metadata)].maximum
                )
                && Supply[object::object_address<Metadata>(self.metadata)].current
                    == old(
                        Supply[object::object_address<Metadata>(self.metadata)].current
                    ) - (self.amount as u128);
        ensures old(
            exists<Supply>(object::object_address<Metadata>(self.metadata))
        ) && old(
            exists<ConcurrentSupply>(object::object_address<Metadata>(self.metadata))
        ) ==>
            Supply[object::object_address<Metadata>(self.metadata)]
                == old(
                    Supply[object::object_address<Metadata>(self.metadata)]
                );
        aborts_if aborts_of<decrease_supply<Metadata>> (self.metadata, self.amount);
    }

    spec decrease_supply<T: key>(
        metadata: &0x1::object::Object<T>, amount: u64
    ) {
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        modifies ConcurrentSupply[object::object_address<T>(metadata)];
        modifies Supply[object::object_address<T>(metadata)];
        ensures exists<ConcurrentSupply>(object::object_address<T>(metadata))
            == old(
                exists<ConcurrentSupply>(object::object_address<T>(metadata))
            );
        ensures exists<Supply>(object::object_address<T>(metadata))
            == old(
                exists<Supply>(object::object_address<T>(metadata))
            );
        ensures old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            aggregator_v2::spec_get_value(
                ConcurrentSupply[object::object_address<T>(metadata)].current
            ) == aggregator_v2::spec_get_value(
                old(
                    ConcurrentSupply[object::object_address<T>(metadata)].current
                )
            ) - (amount as u128);
        ensures old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            aggregator_v2::spec_get_max_value(
                ConcurrentSupply[object::object_address<T>(metadata)].current
            ) == aggregator_v2::spec_get_max_value(
                old(
                    ConcurrentSupply[object::object_address<T>(metadata)].current
                )
            );
        ensures old(
            exists<Supply>(object::object_address<T>(metadata))
        ) && !old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            Supply[object::object_address<T>(metadata)].maximum
                == old(
                    Supply[object::object_address<T>(metadata)].maximum
                );
        ensures old(
            exists<Supply>(object::object_address<T>(metadata))
        ) && !old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            Supply[object::object_address<T>(metadata)].current
                == old(
                    Supply[object::object_address<T>(metadata)].current
                ) - (amount as u128);
        ensures old(
            exists<Supply>(object::object_address<T>(metadata))
        ) && old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            Supply[object::object_address<T>(metadata)]
                == old(Supply[object::object_address<T>(metadata)]);
        aborts_if amount != 0
            && exists<ConcurrentSupply>(object::object_address<T>(metadata))
            && aggregator_v2::spec_get_value(
                ConcurrentSupply[object::object_address<T>(metadata)].current
            ) < (amount as u128);
        aborts_if amount != 0
            && !exists<ConcurrentSupply>(object::object_address<T>(metadata))
            && !exists<Supply>(object::object_address<T>(metadata));
        aborts_if amount != 0
            && !exists<ConcurrentSupply>(object::object_address<T>(metadata))
            && exists<Supply>(object::object_address<T>(metadata))
            && Supply[object::object_address<T>(metadata)].current < (amount as u128);
    }

    spec deposit_dispatch_function<T: key>(
        store: 0x1::object::Object<T>
    ): 0x1::option::Option<0x1::function_info::FunctionInfo> {
        use 0x1::option;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque = true;
        ensures [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && exists<DispatchFunctionStore>(
                object::object_address<Metadata>(
                    FungibleStore[object::object_address<T>(store)].metadata
                )
            ) ==>
            result
                == DispatchFunctionStore[
                    object::object_address<Metadata>(
                        FungibleStore[object::object_address<T>(store)].metadata
                    )
                ].deposit_function;
        ensures [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && !exists<DispatchFunctionStore>(
                object::object_address<Metadata>(
                    FungibleStore[object::object_address<T>(store)].metadata
                )
            ) ==>
            result == option::none<function_info::FunctionInfo>();
        aborts_if [inferred]!exists<FungibleStore>(object::object_address<T>(store));
    }

    spec deposit_sanity_check<T: key>(
        store: 0x1::object::Object<T>, abort_on_dispatch: bool
    ) {
        use 0x1::error;
        use 0x1::object;
        pragma opaque = true;
        aborts_if [inferred]!exists<FungibleStore>(object::object_address<T>(store));
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                !abort_on_dispatch
                    && FungibleStore[object::object_address<T>(store)].frozen
            );
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                !abort_on_dispatch
                    && (
                        FungibleStore[object::object_address<T>(store)].frozen
                            && aborts_of<error::permission_denied>(3)
                    )
            );
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                abort_on_dispatch
                    && result_of<has_deposit_dispatch_function>(
                        FungibleStore[object::object_address<T>(store)].metadata
                    )
            );
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                abort_on_dispatch
                    && (
                        result_of<has_deposit_dispatch_function>(
                            FungibleStore[object::object_address<T>(store)].metadata
                        )
                            && aborts_of<error::invalid_argument>(28)
                    )
            );
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                abort_on_dispatch
                    && (
                        !result_of<has_deposit_dispatch_function>(
                            FungibleStore[object::object_address<T>(store)].metadata
                        )
                            && FungibleStore[object::object_address<T>(store)].frozen
                    )
            );
        aborts_if [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && (
                abort_on_dispatch
                    && (
                        !result_of<has_deposit_dispatch_function>(
                            FungibleStore[object::object_address<T>(store)].metadata
                        )
                            && (
                                FungibleStore[object::object_address<T>(store)].frozen
                                    && aborts_of<error::permission_denied>(3)
                            )
                    )
            );
    }

    spec deposit_with_ref<T: key>(
        self: &0x1::fungible_asset::TransferRef,
        store: 0x1::object::Object<T>,
        fa: 0x1::fungible_asset::FungibleAsset
    ) {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] self.metadata == fa.metadata ==>
            ensures_of<unchecked_deposit>(object::object_address<T>(store), fa);
        aborts_if [inferred] self.metadata == fa.metadata
            && aborts_of<unchecked_deposit>(object::object_address<T>(store), fa);
        aborts_if [inferred] self.metadata != fa.metadata;
    }

    spec derived_balance_dispatch_function<T: key>(
        store: 0x1::object::Object<T>
    ): 0x1::option::Option<0x1::function_info::FunctionInfo> {
        use 0x1::option;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque = true;
        ensures [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && exists<DispatchFunctionStore>(
                object::object_address<Metadata>(
                    FungibleStore[object::object_address<T>(store)].metadata
                )
            ) ==>
            result
                == DispatchFunctionStore[
                    object::object_address<Metadata>(
                        FungibleStore[object::object_address<T>(store)].metadata
                    )
                ].derived_balance_function;
        ensures [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && !exists<DispatchFunctionStore>(
                object::object_address<Metadata>(
                    FungibleStore[object::object_address<T>(store)].metadata
                )
            ) ==>
            result == option::none<function_info::FunctionInfo>();
        aborts_if [inferred]!exists<FungibleStore>(object::object_address<T>(store));
    }

    spec derived_supply_dispatch_function<T: key>(
        metadata: 0x1::object::Object<T>
    ): 0x1::option::Option<0x1::function_info::FunctionInfo> {
        use 0x1::option;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque = true;
        ensures [inferred] exists<DeriveSupply>(object::object_address<T>(metadata)) ==>
            result
                == DeriveSupply[object::object_address<T>(metadata)].dispatch_function;
        ensures [inferred]!exists<DeriveSupply>(object::object_address<T>(metadata)) ==>
            result == option::none<function_info::FunctionInfo>();
        aborts_if [inferred] false;
    }

    spec ensure_store_upgraded_to_concurrent_internal(
        fungible_store_address: address
    ) {
        use 0x1::signer;
        use 0x1::create_signer;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        modifies ConcurrentFungibleBalance[
            signer::address_of(create_signer::spec_create_signer(fungible_store_address))
        ];
        modifies FungibleStore[fungible_store_address];
        ensures [inferred = sathard]!old(
            exists<ConcurrentFungibleBalance>(fungible_store_address)
        ) ==>
            publish<ConcurrentFungibleBalance>(
                signer::address_of(
                    create_signer::spec_create_signer(fungible_store_address)
                ),
                ConcurrentFungibleBalance {
                    balance: result_of<aggregator_v2::create_unbounded_aggregator_with_value<u64
                        >> (old(FungibleStore[fungible_store_address]).balance)
                }
            );
        ensures [inferred]!old(exists<ConcurrentFungibleBalance>(fungible_store_address)) ==>
            FungibleStore[fungible_store_address].balance == 0;
        aborts_if [inferred]!exists<ConcurrentFungibleBalance>(fungible_store_address);
    }

    spec generate_burn_copy_ref(self: &0x1::fungible_asset::BurnRef)
        : 0x1::fungible_asset::BurnRef {
        pragma opaque = true;
        ensures [inferred] result == BurnRef { metadata: self.metadata };
        aborts_if [inferred] false;
    }

    spec generate_burn_ref(
        constructor_ref: &0x1::object::ConstructorRef
    ): 0x1::fungible_asset::BurnRef {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] result
            == BurnRef {
                metadata: object::Object<Metadata> { inner: constructor_ref.self }
            };
        aborts_if [inferred] aborts_of<object::object_from_constructor_ref<Metadata>> (
            constructor_ref
        );
    }

    spec generate_mint_copy_ref(self: &0x1::fungible_asset::MintRef)
        : 0x1::fungible_asset::MintRef {
        pragma opaque = true;
        ensures [inferred] result == MintRef { metadata: self.metadata };
        aborts_if [inferred] false;
    }

    spec generate_mint_ref(
        constructor_ref: &0x1::object::ConstructorRef
    ): 0x1::fungible_asset::MintRef {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] result
            == MintRef {
                metadata: object::Object<Metadata> { inner: constructor_ref.self }
            };
        aborts_if [inferred] aborts_of<object::object_from_constructor_ref<Metadata>> (
            constructor_ref
        );
    }

    spec generate_mutate_metadata_ref(
        constructor_ref: &0x1::object::ConstructorRef
    ): 0x1::fungible_asset::MutateMetadataRef {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] result
            == MutateMetadataRef {
                metadata: object::Object<Metadata> { inner: constructor_ref.self }
            };
        aborts_if [inferred] aborts_of<object::object_from_constructor_ref<Metadata>> (
            constructor_ref
        );
    }

    spec generate_raw_balance_ref(
        constructor_ref: &0x1::object::ConstructorRef
    ): 0x1::fungible_asset::RawBalanceRef {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] result
            == RawBalanceRef {
                metadata: object::Object<Metadata> { inner: constructor_ref.self }
            };
        aborts_if [inferred] aborts_of<object::object_from_constructor_ref<Metadata>> (
            constructor_ref
        );
    }

    spec generate_raw_supply_ref(
        constructor_ref: &0x1::object::ConstructorRef
    ): 0x1::fungible_asset::RawSupplyRef {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] result
            == RawSupplyRef {
                metadata: object::Object<Metadata> { inner: constructor_ref.self }
            };
        aborts_if [inferred] aborts_of<object::object_from_constructor_ref<Metadata>> (
            constructor_ref
        );
    }

    spec has_balance_dispatch_function(
        metadata: 0x1::object::Object<0x1::fungible_asset::Metadata>
    ): bool {
        use 0x1::option;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque = true;
        ensures [inferred] object::object_address<Metadata>(metadata) != @0xa
            && exists<DispatchFunctionStore>(object::object_address<Metadata>(metadata)) ==>
            result
                == option::is_some<function_info::FunctionInfo>(
                    DispatchFunctionStore[object::object_address<Metadata>(metadata)].derived_balance_function
                );
        ensures [inferred] object::object_address<Metadata>(metadata) != @0xa
            && !exists<DispatchFunctionStore>(object::object_address<Metadata>(metadata)) ==> result
            == false;
        ensures [inferred] object::object_address<Metadata>(metadata) == @0xa ==> result
            == false;
        aborts_if [inferred] false;
    }

    spec has_deposit_dispatch_function(
        metadata: 0x1::object::Object<0x1::fungible_asset::Metadata>
    ): bool {
        use 0x1::option;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque = true;
        ensures [inferred] object::object_address<Metadata>(metadata) != @0xa
            && exists<DispatchFunctionStore>(object::object_address<Metadata>(metadata)) ==>
            result
                == option::is_some<function_info::FunctionInfo>(
                    DispatchFunctionStore[object::object_address<Metadata>(metadata)].deposit_function
                );
        ensures [inferred] object::object_address<Metadata>(metadata) != @0xa
            && !exists<DispatchFunctionStore>(object::object_address<Metadata>(metadata)) ==> result
            == false;
        ensures [inferred] object::object_address<Metadata>(metadata) == @0xa ==> result
            == false;
        aborts_if [inferred] false;
    }

    spec has_supply_dispatch_function(metadata_addr: address): bool {
        pragma opaque = true;
        ensures [inferred] metadata_addr != @0xa ==>
            result == exists<DeriveSupply>(metadata_addr);
        ensures [inferred] metadata_addr == @0xa ==> result == false;
        aborts_if [inferred] false;
    }

    spec has_withdraw_dispatch_function(
        metadata: 0x1::object::Object<0x1::fungible_asset::Metadata>
    ): bool {
        use 0x1::option;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque = true;
        ensures [inferred] object::object_address<Metadata>(metadata) != @0xa
            && exists<DispatchFunctionStore>(object::object_address<Metadata>(metadata)) ==>
            result
                == option::is_some<function_info::FunctionInfo>(
                    DispatchFunctionStore[object::object_address<Metadata>(metadata)].withdraw_function
                );
        ensures [inferred] object::object_address<Metadata>(metadata) != @0xa
            && !exists<DispatchFunctionStore>(object::object_address<Metadata>(metadata)) ==> result
            == false;
        ensures [inferred] object::object_address<Metadata>(metadata) == @0xa ==> result
            == false;
        aborts_if [inferred] false;
    }

    spec increase_supply<T: key>(
        metadata: &0x1::object::Object<T>, amount: u64
    ) {
        use 0x1::option;
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        modifies ConcurrentSupply[object::object_address<T>(metadata)];
        modifies Supply[object::object_address<T>(metadata)];
        ensures exists<ConcurrentSupply>(object::object_address<T>(metadata))
            == old(
                exists<ConcurrentSupply>(object::object_address<T>(metadata))
            );
        ensures exists<Supply>(object::object_address<T>(metadata))
            == old(
                exists<Supply>(object::object_address<T>(metadata))
            );
        ensures old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            aggregator_v2::spec_get_value(
                ConcurrentSupply[object::object_address<T>(metadata)].current
            ) == aggregator_v2::spec_get_value(
                old(
                    ConcurrentSupply[object::object_address<T>(metadata)].current
                )
            ) + (amount as u128);
        ensures old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            aggregator_v2::spec_get_max_value(
                ConcurrentSupply[object::object_address<T>(metadata)].current
            ) == aggregator_v2::spec_get_max_value(
                old(
                    ConcurrentSupply[object::object_address<T>(metadata)].current
                )
            );
        ensures old(
            exists<Supply>(object::object_address<T>(metadata))
        ) && !old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            Supply[object::object_address<T>(metadata)].maximum
                == old(
                    Supply[object::object_address<T>(metadata)].maximum
                );
        ensures old(
            exists<Supply>(object::object_address<T>(metadata))
        ) && !old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            Supply[object::object_address<T>(metadata)].current
                == old(
                    Supply[object::object_address<T>(metadata)].current
                ) + (amount as u128);
        ensures old(
            exists<Supply>(object::object_address<T>(metadata))
        ) && old(
            exists<ConcurrentSupply>(object::object_address<T>(metadata))
        ) ==>
            Supply[object::object_address<T>(metadata)]
                == old(Supply[object::object_address<T>(metadata)]);
        aborts_if amount != 0
            && exists<ConcurrentSupply>(object::object_address<T>(metadata))
            && aggregator_v2::spec_get_value(
                ConcurrentSupply[object::object_address<T>(metadata)].current
            ) + (amount as u128)
                > aggregator_v2::spec_get_max_value(
                    ConcurrentSupply[object::object_address<T>(metadata)].current
                );
        aborts_if amount != 0
            && !exists<ConcurrentSupply>(object::object_address<T>(metadata))
            && !exists<Supply>(object::object_address<T>(metadata));
        aborts_if amount != 0
            && !exists<ConcurrentSupply>(object::object_address<T>(metadata))
            && exists<Supply>(object::object_address<T>(metadata))
            && option::is_some(
                Supply[object::object_address<T>(metadata)].maximum
            )
            && option::spec_borrow(
                Supply[object::object_address<T>(metadata)].maximum
            ) < Supply[object::object_address<T>(metadata)].current + (amount as u128);
        aborts_if amount != 0
            && !exists<ConcurrentSupply>(object::object_address<T>(metadata))
            && exists<Supply>(object::object_address<T>(metadata))
            && !option::is_some(
                Supply[object::object_address<T>(metadata)].maximum
            )
            && Supply[object::object_address<T>(metadata)].current + (amount as u128)
                > MAX_U128;
    }

    spec is_address_balance_at_least(store_addr: address, amount: u64): bool {
        use 0x1::aggregator_v2;
        pragma opaque = true;
        ensures [inferred]!exists<FungibleStore>(store_addr) ==>
            result == (amount == 0);
        ensures [inferred = sathard] exists<FungibleStore>(store_addr)
            && FungibleStore[store_addr].balance == 0 ==>
            result
                == result_of<aggregator_v2::is_at_least<u64>> (
                    ConcurrentFungibleBalance[store_addr].balance, amount
                );
        ensures [inferred] exists<FungibleStore>(store_addr)
            && FungibleStore[store_addr].balance != 0 ==>
            result == (FungibleStore[store_addr].balance >= amount);
        aborts_if [inferred] false;
    }

    spec is_asset_type_dispatchable(
        metadata: 0x1::object::Object<0x1::fungible_asset::Metadata>
    ): bool {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] exists<DispatchFunctionStore>(
            object::object_address<Metadata>(metadata)
        ) ==> result == true;
        ensures [inferred]!exists<DispatchFunctionStore>(
            object::object_address<Metadata>(metadata)
        ) ==> result == false;
        aborts_if [inferred] false;
    }

    spec is_balance_at_least<T: key>(
        store: 0x1::object::Object<T>, amount: u64
    ): bool {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] result
            == result_of<is_address_balance_at_least>(
                object::object_address<T>(store), amount
            );
        aborts_if [inferred] false;
    }

    spec is_concurrent<T: key>(store: 0x1::object::Object<T>): bool {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] result
            == exists<ConcurrentFungibleBalance>(object::object_address<T>(store));
        aborts_if [inferred] false;
    }

    spec is_store_dispatchable<T: key>(store: 0x1::object::Object<T>): bool {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] exists<FungibleStore>(object::object_address<T>(store)) ==>
            result
                == exists<DispatchFunctionStore>(
                    object::object_address<Metadata>(
                        FungibleStore[object::object_address<T>(store)].metadata
                    )
                );
        aborts_if [inferred]!exists<FungibleStore>(object::object_address<T>(store));
    }

    spec maximum<T: key>(metadata: 0x1::object::Object<T>): 0x1::option::Option<u128> {
        use 0x1::option;
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] exists<ConcurrentSupply>(object::object_address<T>(metadata))
            && aggregator_v2::max_value<u128>(
                ConcurrentSupply[object::object_address<T>(metadata)].current
            ) == MAX_U128 ==>
            result == option::none<u128>();
        ensures [inferred] exists<ConcurrentSupply>(object::object_address<T>(metadata))
            && aggregator_v2::max_value<u128>(
                ConcurrentSupply[object::object_address<T>(metadata)].current
            ) != MAX_U128 ==>
            result
                == option::some<u128>(
                    aggregator_v2::max_value<u128>(
                        ConcurrentSupply[object::object_address<T>(metadata)].current
                    )
                );
        ensures [inferred]!exists<ConcurrentSupply>(object::object_address<T>(metadata)) ==>
            result == option::none<u128>();
    }

    spec mint(self: &0x1::fungible_asset::MintRef, amount: u64)
        : 0x1::fungible_asset::FungibleAsset {
        use 0x1::object;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies ConcurrentSupply[object::object_address<Metadata>(self.metadata)];
        ensures [inferred] result == result_of<mint_internal>(self.metadata, amount);
    }

    spec mint_internal(
        metadata: 0x1::object::Object<0x1::fungible_asset::Metadata>, amount: u64
    ): 0x1::fungible_asset::FungibleAsset {
        use 0x1::object;
        pragma opaque = true, aborts_if_is_partial = false;
        modifies ConcurrentSupply[object::object_address<Metadata>(metadata)];
        modifies Supply[object::object_address<Metadata>(metadata)];
        ensures [inferred] result == FungibleAsset { metadata: metadata, amount: amount };
        ensures [inferred] ensures_of<increase_supply<Metadata>> (metadata, amount);
        aborts_if aborts_of<increase_supply<Metadata>> (metadata, amount);
    }

    spec mint_to<T: key>(
        self: &0x1::fungible_asset::MintRef, store: 0x1::object::Object<T>, amount: u64
    ) {
        use 0x1::object;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a = S1..S2 |~ result_of<mint>(self, amount);
            S2.. |~ ensures_of<unchecked_deposit>(object::object_address<T>(store), a)
        });
        ensures [inferred]..S1 |~(ensures_of<deposit_sanity_check<T>> (store, false));
        aborts_if [inferred]({
            let a = S1..S2 |~ result_of<mint>(self, amount);
            S2 |~ aborts_of<unchecked_deposit>(object::object_address<T>(store), a)
        });
        aborts_if [inferred] aborts_of<deposit_sanity_check<T>> (store, false);
    }

    spec mutate_metadata(
        self: &0x1::fungible_asset::MutateMetadataRef,
        name: 0x1::option::Option<0x1::string::String>,
        symbol: 0x1::option::Option<0x1::string::String>,
        decimals: 0x1::option::Option<u8>,
        icon_uri: 0x1::option::Option<0x1::string::String>,
        project_uri: 0x1::option::Option<0x1::string::String>
    ) {
        use 0x1::option;
        use 0x1::string;
        use 0x1::error;
        use 0x1::object;
        pragma opaque = true;
        modifies Metadata[object::object_address<Metadata>(self.metadata)];
        ensures [inferred]!option::is_some<string::String>(name)
            && (
                !option::is_some<string::String>(symbol)
                    && (
                        !option::is_some<u8>(decimals)
                            && (
                                !option::is_some<string::String>(icon_uri)
                                    && !option::is_some<string::String>(project_uri)
                            )
                    )
            ) ==>
            update<Metadata>(
                object::object_address<Metadata>(self.metadata),
                old(
                    Metadata[object::object_address<Metadata>(self.metadata)]
                )
            );
        ensures [inferred]({
            let a = ..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    !option::is_some<string::String>(icon_uri)
                                        && (
                                            option::is_some<string::String>(project_uri)
                                                && string::length(a) <= 512
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == a
        });
        ensures [inferred]!option::is_some<string::String>(name)
            && (
                !option::is_some<string::String>(symbol)
                    && (
                        !option::is_some<u8>(decimals)
                            && (
                                !option::is_some<string::String>(icon_uri)
                                    && (
                                        option::is_some<string::String>(project_uri)
                                            && string::length(
                                                ..S8 |~ result_of<option::extract<string::String
                                                >> (project_uri)
                                            ) > 512
                                    )
                            )
                    )
            ) ==>
            {
                let a = object::object_address<Metadata>(self.metadata);
                let b = old(
                    Metadata[object::object_address<Metadata>(self.metadata)]
                );
                S8.. |~ update<Metadata>(a, b)
            };
        ensures [inferred]!option::is_some<string::String>(name)
            && (
                !option::is_some<string::String>(symbol)
                    && (
                        !option::is_some<u8>(decimals)
                            && (
                                option::is_some<string::String>(icon_uri)
                                    && string::length(
                                        ..S6 |~ result_of<option::extract<string::String>> (
                                            icon_uri
                                        )
                                    ) > 512
                            )
                    )
            ) ==>
            {
                let a = object::object_address<Metadata>(self.metadata);
                let b = old(
                    Metadata[object::object_address<Metadata>(self.metadata)]
                );
                S6.. |~ update<Metadata>(a, b)
            };
        ensures [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    option::is_some<string::String>(icon_uri)
                                        && (
                                            string::length(a) <= 512
                                                && !option::is_some<string::String>(
                                                    project_uri
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        old(
                            Metadata[object::object_address<Metadata>(self.metadata)]
                        ),
                        icon_uri,
                        a
                    )
                )
        });
        ensures [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    option::is_some<string::String>(icon_uri)
                                        && (
                                            string::length(a) <= 512
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(b) <= 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == b
        });
        ensures [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    option::is_some<string::String>(icon_uri)
                                        && (
                                            string::length(a) <= 512
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].icon_uri == a
        });
        ensures [inferred]!option::is_some<string::String>(name)
            && (
                !option::is_some<string::String>(symbol)
                    && (
                        option::is_some<u8>(decimals)
                            && (..S4 |~ result_of<option::extract<u8>> (decimals) > 32)
                    )
            ) ==>
            {
                let a = object::object_address<Metadata>(self.metadata);
                let b = old(
                    Metadata[object::object_address<Metadata>(self.metadata)]
                );
                S4.. |~ update<Metadata>(a, b)
            };
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && !option::is_some<string::String>(
                                                    project_uri
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        old(
                            Metadata[object::object_address<Metadata>(self.metadata)]
                        ),
                        decimals,
                        a
                    )
                )
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(b) <= 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == b
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].decimals == a
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && string::length(b) > 512
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].decimals == a
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && !option::is_some<string::String>(
                                                            project_uri
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            old(
                                Metadata[object::object_address<Metadata>(self.metadata)]
                            ),
                            decimals,
                            a
                        ),
                        icon_uri,
                        b
                    )
                )
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(c)
                                                                    <= 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == c
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(c)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].icon_uri == b
        });
        ensures [inferred]!option::is_some<string::String>(name)
            && (
                option::is_some<string::String>(symbol)
                    && string::length(
                        ..S2 |~ result_of<option::extract<string::String>> (symbol)
                    ) > 32
            ) ==>
            {
                let a = object::object_address<Metadata>(self.metadata);
                let b = old(
                    Metadata[object::object_address<Metadata>(self.metadata)]
                );
                S2.. |~ update<Metadata>(a, b)
            };
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && !option::is_some<string::String>(
                                                    project_uri
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        old(
                            Metadata[object::object_address<Metadata>(self.metadata)]
                        ),
                        symbol,
                        a
                    )
                )
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(b) <= 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == b
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].symbol == a
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && string::length(b) > 512
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].symbol == a
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && !option::is_some<string::String>(
                                                            project_uri
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            old(
                                Metadata[object::object_address<Metadata>(self.metadata)]
                            ),
                            symbol,
                            a
                        ),
                        icon_uri,
                        b
                    )
                )
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(c)
                                                                    <= 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == c
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(c)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].icon_uri == b
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (string::length(a) <= 32
                            && (option::is_some<u8>(decimals)
                                && b > 32))
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].symbol == a
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && !option::is_some<string::String>(
                                                            project_uri
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            old(
                                Metadata[object::object_address<Metadata>(self.metadata)]
                            ),
                            symbol,
                            a
                        ),
                        decimals,
                        b
                    )
                )
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(c)
                                                                    <= 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == c
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(c)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].decimals == b
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && string::length(c) > 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].decimals == b
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(c) <= 512
                                                                && !option::is_some<string::String>(
                                                                    project_uri
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            update_field(
                                old(
                                    Metadata[object::object_address<Metadata>(
                                        self.metadata
                                    )]
                                ),
                                symbol,
                                a
                            ),
                            decimals,
                            b
                        ),
                        icon_uri,
                        c
                    )
                )
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(c) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            d
                                                                        ) <= 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == d
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(c) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            d
                                                                        ) > 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].icon_uri == c
        });
        ensures [inferred] option::is_some<string::String>(name)
            && (
                string::length(result_of<option::extract<string::String>> (name)) <= 32
                    && (
                        !option::is_some<string::String>(symbol)
                            && (
                                !option::is_some<u8>(decimals)
                                    && (
                                        !option::is_some<string::String>(icon_uri)
                                            && !option::is_some<string::String>(
                                                project_uri
                                            )
                                    )
                            )
                    )
            ) ==>
            update<Metadata>(
                object::object_address<Metadata>(self.metadata),
                update_field(
                    old(
                        Metadata[object::object_address<Metadata>(self.metadata)]
                    ),
                    name,
                    result_of<option::extract<string::String>> (name)
                )
            );
        ensures [inferred]({
            let a = ..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(a) <= 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == a
        });
        ensures [inferred]({
            let a = ..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(a) > 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].name
                    == result_of<option::extract<string::String>> (name)
        });
        ensures [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && string::length(a) > 512
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].name
                    == result_of<option::extract<string::String>> (name)
        });
        ensures [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(a) <= 512
                                                        && !option::is_some<string::String>(
                                                            project_uri
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            old(
                                Metadata[object::object_address<Metadata>(self.metadata)]
                            ),
                            name,
                            result_of<option::extract<string::String>> (name)
                        ),
                        icon_uri,
                        a
                    )
                )
        });
        ensures [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(a) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(b)
                                                                    <= 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == b
        });
        ensures [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(a) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(b)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].icon_uri == a
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (option::is_some<u8>(decimals)
                                    && a > 32)
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].name
                    == result_of<option::extract<string::String>> (name)
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && !option::is_some<string::String>(
                                                            project_uri
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            old(
                                Metadata[object::object_address<Metadata>(self.metadata)]
                            ),
                            name,
                            result_of<option::extract<string::String>> (name)
                        ),
                        decimals,
                        a
                    )
                )
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(b)
                                                                    <= 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == b
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(b)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].decimals == a
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].decimals == a
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && !option::is_some<string::String>(
                                                                    project_uri
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            update_field(
                                old(
                                    Metadata[object::object_address<Metadata>(
                                        self.metadata
                                    )]
                                ),
                                name,
                                result_of<option::extract<string::String>> (name)
                            ),
                            decimals,
                            a
                        ),
                        icon_uri,
                        b
                    )
                )
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            c
                                                                        ) <= 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == c
        });
        ensures [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            c
                                                                        ) > 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].icon_uri == b
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (option::is_some<string::String>(symbol)
                            && string::length(a) > 32)
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].name
                    == result_of<option::extract<string::String>> (name)
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && !option::is_some<string::String>(
                                                            project_uri
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            old(
                                Metadata[object::object_address<Metadata>(self.metadata)]
                            ),
                            name,
                            result_of<option::extract<string::String>> (name)
                        ),
                        symbol,
                        a
                    )
                )
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(b)
                                                                    <= 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == b
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(b)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].symbol == a
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].symbol == a
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && !option::is_some<string::String>(
                                                                    project_uri
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            update_field(
                                old(
                                    Metadata[object::object_address<Metadata>(
                                        self.metadata
                                    )]
                                ),
                                name,
                                result_of<option::extract<string::String>> (name)
                            ),
                            symbol,
                            a
                        ),
                        icon_uri,
                        b
                    )
                )
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            c
                                                                        ) <= 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == c
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            c
                                                                        ) > 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].icon_uri == b
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (option::is_some<u8>(decimals)
                                            && b > 32)
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].symbol == a
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            !option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && !option::is_some<string::String>(
                                                                    project_uri
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            update_field(
                                old(
                                    Metadata[object::object_address<Metadata>(
                                        self.metadata
                                    )]
                                ),
                                name,
                                result_of<option::extract<string::String>> (name)
                            ),
                            symbol,
                            a
                        ),
                        decimals,
                        b
                    )
                )
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            !option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            c
                                                                        ) <= 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == c
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            !option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            c
                                                                        ) > 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].decimals == b
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && string::length(c)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].decimals == b
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        <= 512
                                                                        && !option::is_some<string::String>(
                                                                            project_uri
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                update<Metadata>(
                    object::object_address<Metadata>(self.metadata),
                    update_field(
                        update_field(
                            update_field(
                                update_field(
                                    old(
                                        Metadata[object::object_address<Metadata>(
                                            self.metadata
                                        )]
                                    ),
                                    name,
                                    result_of<option::extract<string::String>> (name)
                                ),
                                symbol,
                                a
                            ),
                            decimals,
                            b
                        ),
                        icon_uri,
                        c
                    )
                )
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        <= 512
                                                                        && (
                                                                            option::is_some<string::String>(
                                                                                project_uri
                                                                            )
                                                                                && string::length(
                                                                                    d
                                                                                ) <= 512
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].project_uri
                    == d
        });
        ensures [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        <= 512
                                                                        && (
                                                                            option::is_some<string::String>(
                                                                                project_uri
                                                                            )
                                                                                && string::length(
                                                                                    d
                                                                                ) > 512
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                ) ==>
                Metadata[object::object_address<Metadata>(self.metadata)].icon_uri == c
        });
        aborts_if [inferred]({
            let a = ..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    !option::is_some<string::String>(icon_uri)
                                        && (
                                            option::is_some<string::String>(project_uri)
                                                && string::length(a) > 512
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    !option::is_some<string::String>(icon_uri)
                                        && (
                                            option::is_some<string::String>(project_uri)
                                                && (
                                                    string::length(a) > 512
                                                        && aborts_of<error::out_of_range>(
                                                        19)
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]!option::is_some<string::String>(name)
            && (
                !option::is_some<string::String>(symbol)
                    && (
                        !option::is_some<u8>(decimals)
                            && (
                                !option::is_some<string::String>(icon_uri)
                                    && (
                                        option::is_some<string::String>(project_uri)
                                            && aborts_of<option::extract<string::String>> (
                                                project_uri
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    option::is_some<string::String>(icon_uri)
                                        && string::length(a) > 512
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    option::is_some<string::String>(icon_uri)
                                        && (
                                            string::length(a) > 512
                                                && aborts_of<error::out_of_range>(19)
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    option::is_some<string::String>(icon_uri)
                                        && (
                                            string::length(a) <= 512
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    option::is_some<string::String>(icon_uri)
                                        && (
                                            string::length(a) <= 512
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && (
                                                            string::length(b) > 512
                                                                && aborts_of<error::out_of_range>(
                                                                    19
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6 |~ aborts_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            !option::is_some<u8>(decimals)
                                && (
                                    option::is_some<string::String>(icon_uri)
                                        && (
                                            string::length(a) <= 512
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && b
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]!option::is_some<string::String>(name)
            && (
                !option::is_some<string::String>(symbol)
                    && (
                        !option::is_some<u8>(decimals)
                            && (
                                option::is_some<string::String>(icon_uri)
                                    && aborts_of<option::extract<string::String>> (
                                        icon_uri
                                    )
                            )
                    )
            );
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (option::is_some<u8>(decimals)
                            && a > 32)
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (a > 32
                                    && aborts_of<error::out_of_range>(17))
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && (
                                                            string::length(b) > 512
                                                                && aborts_of<error::out_of_range>(
                                                                    19
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4 |~ aborts_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && b
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && string::length(b) > 512
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) > 512
                                                        && aborts_of<error::out_of_range>(
                                                        19)
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(c)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        > 512
                                                                        && aborts_of<error::out_of_range>(
                                                                            19
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6 |~ aborts_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (
                                    a <= 32
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && c
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4 |~ aborts_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    !option::is_some<string::String>(symbol)
                        && (
                            option::is_some<u8>(decimals)
                                && (a <= 32
                                    && (option::is_some<string::String>(icon_uri)
                                        && b))
                        )
                )
        });
        aborts_if [inferred]!option::is_some<string::String>(name)
            && (
                !option::is_some<string::String>(symbol)
                    && (
                        option::is_some<u8>(decimals)
                            && aborts_of<option::extract<u8>> (decimals)
                    )
            );
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            !option::is_some<string::String>(name)
                && (option::is_some<string::String>(symbol) && string::length(a) > 32)
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (string::length(a) > 32
                            && aborts_of<error::out_of_range>(16))
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && (
                                                            string::length(b) > 512
                                                                && aborts_of<error::out_of_range>(
                                                                    19
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2 |~ aborts_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && b
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && string::length(b) > 512
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) > 512
                                                        && aborts_of<error::out_of_range>(
                                                        19)
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(c)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        > 512
                                                                        && aborts_of<error::out_of_range>(
                                                                            19
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6 |~ aborts_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(b) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && c
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2 |~ aborts_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (option::is_some<string::String>(icon_uri)
                                            && b)
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (string::length(a) <= 32
                            && (option::is_some<u8>(decimals)
                                && b > 32))
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (b > 32
                                            && aborts_of<error::out_of_range>(17))
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(c)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        > 512
                                                                        && aborts_of<error::out_of_range>(
                                                                            19
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4 |~ aborts_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && c
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && string::length(c) > 512
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(c) > 512
                                                                && aborts_of<error::out_of_range>(
                                                                    19
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(c) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            d
                                                                        ) > 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(c) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && (
                                                                            string::length(
                                                                                d
                                                                            ) > 512
                                                                                && aborts_of<error::out_of_range>(
                                                                                    19
                                                                                )
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6 |~ aborts_of<option::extract<string::String>> (project_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(c) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && d
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4 |~ aborts_of<option::extract<string::String>> (icon_uri);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (
                            string::length(a) <= 32
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            b <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && c
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2 |~ aborts_of<option::extract<u8>> (decimals);
            !option::is_some<string::String>(name)
                && (
                    option::is_some<string::String>(symbol)
                        && (string::length(a) <= 32
                            && (option::is_some<u8>(decimals)
                                && b))
                )
        });
        aborts_if [inferred]!option::is_some<string::String>(name)
            && (
                option::is_some<string::String>(symbol)
                    && aborts_of<option::extract<string::String>> (symbol)
            );
        aborts_if [inferred] option::is_some<string::String>(name)
            && string::length(result_of<option::extract<string::String>> (name)) > 32;
        aborts_if [inferred] option::is_some<string::String>(name)
            && (
                string::length(result_of<option::extract<string::String>> (name)) > 32
                    && aborts_of<error::out_of_range>(15)
            );
        aborts_if [inferred]({
            let a = ..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && string::length(a) > 512
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            !option::is_some<string::String>(icon_uri)
                                                && (
                                                    option::is_some<string::String>(
                                                        project_uri
                                                    )
                                                        && (
                                                            string::length(a) > 512
                                                                && aborts_of<error::out_of_range>(
                                                                    19
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred] option::is_some<string::String>(name)
            && (
                string::length(result_of<option::extract<string::String>> (name)) <= 32
                    && (
                        !option::is_some<string::String>(symbol)
                            && (
                                !option::is_some<u8>(decimals)
                                    && (
                                        !option::is_some<string::String>(icon_uri)
                                            && (
                                                option::is_some<string::String>(
                                                    project_uri
                                                )
                                                    && aborts_of<option::extract<string::String
                                                    >> (project_uri)
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && string::length(a) > 512
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(a) > 512
                                                        && aborts_of<error::out_of_range>(
                                                        19)
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(a) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(b)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(a) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && (
                                                                    string::length(b)
                                                                        > 512
                                                                        && aborts_of<error::out_of_range>(
                                                                            19
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let b = S6 |~ aborts_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    !option::is_some<u8>(decimals)
                                        && (
                                            option::is_some<string::String>(icon_uri)
                                                && (
                                                    string::length(a) <= 512
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && b
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred] option::is_some<string::String>(name)
            && (
                string::length(result_of<option::extract<string::String>> (name)) <= 32
                    && (
                        !option::is_some<string::String>(symbol)
                            && (
                                !option::is_some<u8>(decimals)
                                    && (
                                        option::is_some<string::String>(icon_uri)
                                            && aborts_of<option::extract<string::String>> (
                                                icon_uri
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (option::is_some<u8>(decimals)
                                    && a > 32)
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (a > 32
                                            && aborts_of<error::out_of_range>(17))
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(b)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && (
                                                                    string::length(b)
                                                                        > 512
                                                                        && aborts_of<error::out_of_range>(
                                                                            19
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4 |~ aborts_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && b
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) > 512
                                                                && aborts_of<error::out_of_range>(
                                                                    19
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            c
                                                                        ) > 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && (
                                                                            string::length(
                                                                                c
                                                                            ) > 512
                                                                                && aborts_of<error::out_of_range>(
                                                                                    19
                                                                                )
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6 |~ aborts_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && c
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S4 |~ result_of<option::extract<u8>> (decimals);
            let b = S4 |~ aborts_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            !option::is_some<string::String>(symbol)
                                && (
                                    option::is_some<u8>(decimals)
                                        && (
                                            a <= 32
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && b
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred] option::is_some<string::String>(name)
            && (
                string::length(result_of<option::extract<string::String>> (name)) <= 32
                    && (
                        !option::is_some<string::String>(symbol)
                            && (
                                option::is_some<u8>(decimals)
                                    && aborts_of<option::extract<u8>> (decimals)
                            )
                    )
            );
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (option::is_some<string::String>(symbol)
                            && string::length(a) > 32)
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) > 32
                                        && aborts_of<error::out_of_range>(16)
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && string::length(b)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && (
                                                                    string::length(b)
                                                                        > 512
                                                                        && aborts_of<error::out_of_range>(
                                                                            19
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2 |~ aborts_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    !option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            option::is_some<string::String>(
                                                                project_uri
                                                            )
                                                                && b
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && string::length(b) > 512
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) > 512
                                                                && aborts_of<error::out_of_range>(
                                                                    19
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            c
                                                                        ) > 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && (
                                                                            string::length(
                                                                                c
                                                                            ) > 512
                                                                                && aborts_of<error::out_of_range>(
                                                                                    19
                                                                                )
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let c = S6 |~ aborts_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && (
                                                            string::length(b) <= 512
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && c
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2 |~ aborts_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            !option::is_some<u8>(decimals)
                                                && (
                                                    option::is_some<string::String>(
                                                        icon_uri
                                                    )
                                                        && b
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (option::is_some<u8>(decimals)
                                            && b > 32)
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b > 32
                                                        && aborts_of<error::out_of_range>(
                                                        17)
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            !option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && string::length(
                                                                            c
                                                                        ) > 512
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            !option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && (
                                                                            string::length(
                                                                                c
                                                                            ) > 512
                                                                                && aborts_of<error::out_of_range>(
                                                                                    19
                                                                                )
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4 |~ aborts_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            !option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    option::is_some<string::String>(
                                                                        project_uri
                                                                    )
                                                                        && c
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && string::length(c)
                                                                    > 512
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        > 512
                                                                        && aborts_of<error::out_of_range>(
                                                                            19
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        <= 512
                                                                        && (
                                                                            option::is_some<string::String>(
                                                                                project_uri
                                                                            )
                                                                                && string::length(
                                                                                    d
                                                                                ) > 512
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6..S8 |~ result_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        <= 512
                                                                        && (
                                                                            option::is_some<string::String>(
                                                                                project_uri
                                                                            )
                                                                                && (
                                                                                    string::length(
                                                                                        d
                                                                                    ) >
                                                                                    512
                                                                                        && aborts_of<error::out_of_range>(

                                                                                            19
                                                                                        )
                                                                                )
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4..S6 |~ result_of<option::extract<string::String>> (icon_uri);
            let d = S6 |~ aborts_of<option::extract<string::String>> (project_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && (
                                                                    string::length(c)
                                                                        <= 512
                                                                        && (
                                                                            option::is_some<string::String>(
                                                                                project_uri
                                                                            )
                                                                                && d
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2..S4 |~ result_of<option::extract<u8>> (decimals);
            let c = S4 |~ aborts_of<option::extract<string::String>> (icon_uri);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (
                                            option::is_some<u8>(decimals)
                                                && (
                                                    b <= 32
                                                        && (
                                                            option::is_some<string::String>(
                                                                icon_uri
                                                            )
                                                                && c
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<option::extract<string::String>> (symbol);
            let b = S2 |~ aborts_of<option::extract<u8>> (decimals);
            option::is_some<string::String>(name)
                && (
                    string::length(result_of<option::extract<string::String>> (name))
                        <= 32
                        && (
                            option::is_some<string::String>(symbol)
                                && (
                                    string::length(a) <= 32
                                        && (option::is_some<u8>(decimals)
                                            && b)
                                )
                        )
                )
        });
        aborts_if [inferred] option::is_some<string::String>(name)
            && (
                string::length(result_of<option::extract<string::String>> (name)) <= 32
                    && (
                        option::is_some<string::String>(symbol)
                            && aborts_of<option::extract<string::String>> (symbol)
                    )
            );
        aborts_if [inferred] option::is_some<string::String>(name)
            && aborts_of<option::extract<string::String>> (name);
        aborts_if [inferred]!exists<Metadata>(
            object::object_address<Metadata>(self.metadata)
        );
    }

    spec register_derive_supply_dispatch_function(
        constructor_ref: &0x1::object::ConstructorRef,
        dispatch_function: 0x1::option::Option<0x1::function_info::FunctionInfo>
    ) {
        use 0x1::option;
        use 0x1::string;
        use 0x1::signer;
        use 0x1::error;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque = true;
        modifies DeriveSupply[
            signer::address_of(result_of<object::generate_signer>(constructor_ref))
        ];
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && object::address_from_constructor_ref(constructor_ref) == @0xa;
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            dispatch_function
        )
            && (
                object::address_from_constructor_ref(constructor_ref) == @0xa
                    && aborts_of<error::permission_denied>(31)
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            dispatch_function
        )
            && (
                object::address_from_constructor_ref(constructor_ref) != @0xa
                    && object::can_generate_delete_ref(constructor_ref)
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            dispatch_function
        )
            && (
                object::address_from_constructor_ref(constructor_ref) != @0xa
                    && (
                        object::can_generate_delete_ref(constructor_ref)
                            && aborts_of<error::invalid_argument>(18)
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && (
            object::address_from_constructor_ref(constructor_ref) != @0xa
                && (
                    !object::can_generate_delete_ref(constructor_ref)
                        && !exists<Metadata>(
                            object::address_from_constructor_ref(constructor_ref)
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && (
            object::address_from_constructor_ref(constructor_ref) != @0xa
                && (
                    !object::can_generate_delete_ref(constructor_ref)
                        && (
                            !exists<Metadata>(
                                object::address_from_constructor_ref(constructor_ref)
                            )
                                && aborts_of<error::not_found>(30)
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S2 |~ exists<DeriveSupply>(
                    signer::address_of(
                        ..S2 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            !option::is_some<function_info::FunctionInfo>(dispatch_function)
                && (
                    object::address_from_constructor_ref(constructor_ref) != @0xa
                        && (
                            !object::can_generate_delete_ref(constructor_ref)
                                && (
                                    exists<Metadata>(
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        )
                                    )
                                        && (
                                            !exists<DeriveSupply>(
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                )
                                            )
                                                && a
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && (
            object::address_from_constructor_ref(constructor_ref) != @0xa
                && (
                    !object::can_generate_delete_ref(constructor_ref)
                        && exists<Metadata>(
                            object::address_from_constructor_ref(constructor_ref)
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && (
            object::address_from_constructor_ref(constructor_ref) != @0xa
                && (
                    !object::can_generate_delete_ref(constructor_ref)
                        && (
                            exists<Metadata>(
                                object::address_from_constructor_ref(constructor_ref)
                            )
                                && (
                                    exists<DeriveSupply>(
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        )
                                    )
                                        && aborts_of<error::already_exists>(29)
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && !result_of<function_info::check_dispatch_type_compatibility>(
            function_info::new_function_info_from_address(
                @0x1,
                string::utf8(
                    vector[
                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 102,
                        117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101, 116
                    ]
                ),
                string::utf8(
                    vector[
                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 100,
                        101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112, 108, 121
                    ]
                )
            ),
            option::borrow<function_info::FunctionInfo>(dispatch_function)
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        )
            && (
                !result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                100, 101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112,
                                108, 121
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(dispatch_function)
                )
                    && aborts_of<error::invalid_argument>(33)
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            100, 101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112, 108,
                            121
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(dispatch_function)
            ) && object::address_from_constructor_ref(constructor_ref) == @0xa
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                100, 101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112,
                                108, 121
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(dispatch_function)
                )
                    && (
                        object::address_from_constructor_ref(constructor_ref) == @0xa
                            && aborts_of<error::permission_denied>(31)
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                100, 101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112,
                                108, 121
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(dispatch_function)
                )
                    && (
                        object::address_from_constructor_ref(constructor_ref) != @0xa
                            && object::can_generate_delete_ref(constructor_ref)
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                100, 101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112,
                                108, 121
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(dispatch_function)
                )
                    && (
                        object::address_from_constructor_ref(constructor_ref) != @0xa
                            && (
                                object::can_generate_delete_ref(constructor_ref)
                                    && aborts_of<error::invalid_argument>(18)
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            100, 101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112, 108,
                            121
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(dispatch_function)
            )
                && (
                    object::address_from_constructor_ref(constructor_ref) != @0xa
                        && (
                            !object::can_generate_delete_ref(constructor_ref)
                                && !exists<Metadata>(
                                    object::address_from_constructor_ref(constructor_ref)
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            100, 101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112, 108,
                            121
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(dispatch_function)
            )
                && (
                    object::address_from_constructor_ref(constructor_ref) != @0xa
                        && (
                            !object::can_generate_delete_ref(constructor_ref)
                                && (
                                    !exists<Metadata>(
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        )
                                    )
                                        && aborts_of<error::not_found>(30)
                                )
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S2 |~ exists<DeriveSupply>(
                    signer::address_of(
                        ..S2 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            option::is_some<function_info::FunctionInfo>(dispatch_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 114, 105, 118, 101, 100, 95, 115, 117,
                                    112, 112, 108, 121
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(dispatch_function)
                    )
                        && (
                            object::address_from_constructor_ref(constructor_ref)
                                != @0xa
                                && (
                                    !object::can_generate_delete_ref(constructor_ref)
                                        && (
                                            exists<Metadata>(
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                )
                                            )
                                                && (
                                                    !exists<DeriveSupply>(
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        )
                                                    )
                                                        && a
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            100, 101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112, 108,
                            121
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(dispatch_function)
            )
                && (
                    object::address_from_constructor_ref(constructor_ref) != @0xa
                        && (
                            !object::can_generate_delete_ref(constructor_ref)
                                && exists<Metadata>(
                                    object::address_from_constructor_ref(constructor_ref)
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            100, 101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112, 108,
                            121
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(dispatch_function)
            )
                && (
                    object::address_from_constructor_ref(constructor_ref) != @0xa
                        && (
                            !object::can_generate_delete_ref(constructor_ref)
                                && (
                                    exists<Metadata>(
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        )
                                    )
                                        && (
                                            exists<DeriveSupply>(
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                )
                                            )
                                                && aborts_of<error::already_exists>(29)
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && aborts_of<function_info::check_dispatch_type_compatibility>(
            function_info::new_function_info_from_address(
                @0x1,
                string::utf8(
                    vector[
                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 102,
                        117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101, 116
                    ]
                ),
                string::utf8(
                    vector[
                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 100,
                        101, 114, 105, 118, 101, 100, 95, 115, 117, 112, 112, 108, 121
                    ]
                )
            ),
            option::borrow<function_info::FunctionInfo>(dispatch_function)
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && aborts_of<function_info::new_function_info_from_address>(
            @0x1,
            string::utf8(
                vector[
                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 102, 117,
                    110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101, 116
                ]
            ),
            string::utf8(
                vector[
                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 100, 101,
                    114, 105, 118, 101, 100, 95, 115, 117, 112, 112, 108, 121
                ]
            )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            dispatch_function
        ) && aborts_of<option::borrow<function_info::FunctionInfo>> (dispatch_function);
    }

    spec register_dispatch_functions(
        constructor_ref: &0x1::object::ConstructorRef,
        withdraw_function: 0x1::option::Option<0x1::function_info::FunctionInfo>,
        deposit_function: 0x1::option::Option<0x1::function_info::FunctionInfo>,
        derived_balance_function: 0x1::option::Option<0x1::function_info::FunctionInfo>
    ) {
        use 0x1::option;
        use 0x1::string;
        use 0x1::signer;
        use 0x1::error;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque = true;
        modifies DispatchFunctionStore[
            signer::address_of(result_of<object::generate_signer>(constructor_ref))
        ];
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                !option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        !option::is_some<function_info::FunctionInfo>(
                            derived_balance_function
                        )
                            && object::address_from_constructor_ref(constructor_ref)
                                == @0xa
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                !option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        !option::is_some<function_info::FunctionInfo>(
                            derived_balance_function
                        )
                            && (
                                object::address_from_constructor_ref(constructor_ref)
                                    == @0xa
                                    && aborts_of<error::permission_denied>(31)
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                !option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        !option::is_some<function_info::FunctionInfo>(
                            derived_balance_function
                        )
                            && (
                                object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                    && object::can_generate_delete_ref(constructor_ref)
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                !option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        !option::is_some<function_info::FunctionInfo>(
                            derived_balance_function
                        )
                            && (
                                object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                    && (
                                        object::can_generate_delete_ref(constructor_ref)
                                            && aborts_of<error::invalid_argument>(18)
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    !option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            object::address_from_constructor_ref(constructor_ref)
                                != @0xa
                                && (
                                    !object::can_generate_delete_ref(constructor_ref)
                                        && !exists<Metadata>(
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    !option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            object::address_from_constructor_ref(constructor_ref)
                                != @0xa
                                && (
                                    !object::can_generate_delete_ref(constructor_ref)
                                        && (
                                            !exists<Metadata>(
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                )
                                            )
                                                && aborts_of<error::not_found>(30)
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S4 |~ exists<DispatchFunctionStore>(
                    signer::address_of(
                        ..S4 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            !option::is_some<function_info::FunctionInfo>(withdraw_function)
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && (
                                                    exists<Metadata>(
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        )
                                                    )
                                                        && (
                                                            !exists<DispatchFunctionStore>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && a
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    !option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            object::address_from_constructor_ref(constructor_ref)
                                != @0xa
                                && (
                                    !object::can_generate_delete_ref(constructor_ref)
                                        && exists<Metadata>(
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    !option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            object::address_from_constructor_ref(constructor_ref)
                                != @0xa
                                && (
                                    !object::can_generate_delete_ref(constructor_ref)
                                        && (
                                            exists<Metadata>(
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                )
                                            )
                                                && (
                                                    exists<DispatchFunctionStore>(
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        )
                                                    )
                                                        && aborts_of<error::already_exists>(
                                                            29
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && !result_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 114, 105, 118, 101, 100, 95, 98,
                                        97, 108, 97, 110, 99, 101
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            !result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 114, 105, 118, 101,
                                            100, 95, 98, 97, 108, 97, 110, 99, 101
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                            )
                                && aborts_of<error::invalid_argument>(27)
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 114, 105, 118, 101,
                                            100, 95, 98, 97, 108, 97, 110, 99, 101
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                            )
                                && object::address_from_constructor_ref(constructor_ref)
                                    == @0xa
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                !option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        option::is_some<function_info::FunctionInfo>(
                            derived_balance_function
                        )
                            && (
                                result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 114, 105, 118,
                                                101, 100, 95, 98, 97, 108, 97, 110, 99,
                                                101
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                )
                                    && (
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        ) == @0xa
                                            && aborts_of<error::permission_denied>(31)
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                !option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        option::is_some<function_info::FunctionInfo>(
                            derived_balance_function
                        )
                            && (
                                result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 114, 105, 118,
                                                101, 100, 95, 98, 97, 108, 97, 110, 99,
                                                101
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                )
                                    && (
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        ) != @0xa
                                            && object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                !option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        option::is_some<function_info::FunctionInfo>(
                            derived_balance_function
                        )
                            && (
                                result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 114, 105, 118,
                                                101, 100, 95, 98, 97, 108, 97, 110, 99,
                                                101
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                )
                                    && (
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        ) != @0xa
                                            && (
                                                object::can_generate_delete_ref(
                                                    constructor_ref
                                                )
                                                    && aborts_of<error::invalid_argument>(
                                                    18)
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 114, 105, 118, 101,
                                            100, 95, 98, 97, 108, 97, 110, 99, 101
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && !exists<Metadata>(
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 114, 105, 118, 101,
                                            100, 95, 98, 97, 108, 97, 110, 99, 101
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && (
                                                    !exists<Metadata>(
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        )
                                                    )
                                                        && aborts_of<error::not_found>(30)
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S4 |~ exists<DispatchFunctionStore>(
                    signer::address_of(
                        ..S4 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            !option::is_some<function_info::FunctionInfo>(withdraw_function)
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && (
                                                            exists<Metadata>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && (
                                                                    !exists<
                                                                        DispatchFunctionStore>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && a
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 114, 105, 118, 101,
                                            100, 95, 98, 97, 108, 97, 110, 99, 101
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && exists<Metadata>(
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 114, 105, 118, 101,
                                            100, 95, 98, 97, 108, 97, 110, 99, 101
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && (
                                                    exists<Metadata>(
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        )
                                                    )
                                                        && (
                                                            exists<DispatchFunctionStore>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && aborts_of<error::already_exists>(
                                                                    29
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && aborts_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 114, 105, 118, 101, 100, 95, 98,
                                        97, 108, 97, 110, 99, 101
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && aborts_of<function_info::new_function_info_from_address>(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 114, 105, 118, 101, 100, 95, 98, 97, 108,
                                    97, 110, 99, 101
                                ]
                            )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            !option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    option::is_some<function_info::FunctionInfo>(derived_balance_function)
                        && aborts_of<option::borrow<function_info::FunctionInfo>> (
                            derived_balance_function
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && !result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                100, 101, 112, 111, 115, 105, 116
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(deposit_function)
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        !result_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 112, 111, 115, 105, 116
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(deposit_function)
                        )
                            && aborts_of<error::invalid_argument>(26)
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && object::address_from_constructor_ref(constructor_ref)
                                    == @0xa
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        result_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 112, 111, 115, 105, 116
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(deposit_function)
                        )
                            && (
                                !option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        ) == @0xa
                                            && aborts_of<error::permission_denied>(31)
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        result_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 112, 111, 115, 105, 116
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(deposit_function)
                        )
                            && (
                                !option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        ) != @0xa
                                            && object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        result_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 112, 111, 115, 105, 116
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(deposit_function)
                        )
                            && (
                                !option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        ) != @0xa
                                            && (
                                                object::can_generate_delete_ref(
                                                    constructor_ref
                                                )
                                                    && aborts_of<error::invalid_argument>(
                                                    18)
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && !exists<Metadata>(
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && (
                                                    !exists<Metadata>(
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        )
                                                    )
                                                        && aborts_of<error::not_found>(30)
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S4 |~ exists<DispatchFunctionStore>(
                    signer::address_of(
                        ..S4 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            !option::is_some<function_info::FunctionInfo>(withdraw_function)
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    !option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && (
                                                            exists<Metadata>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && (
                                                                    !exists<
                                                                        DispatchFunctionStore>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && a
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && exists<Metadata>(
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && (
                                                    exists<Metadata>(
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        )
                                                    )
                                                        && (
                                                            exists<DispatchFunctionStore>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && aborts_of<error::already_exists>(
                                                                    29
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && !result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 114, 105, 118,
                                                101, 100, 95, 98, 97, 108, 97, 110, 99,
                                                101
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    !result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && aborts_of<error::invalid_argument>(27)
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && object::address_from_constructor_ref(
                                            constructor_ref
                                        ) == @0xa
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        result_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 112, 111, 115, 105, 116
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(deposit_function)
                        )
                            && (
                                option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        result_of<function_info::check_dispatch_type_compatibility>(
                                            function_info::new_function_info_from_address(
                                                @0x1,
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 102,
                                                        117, 110, 103, 105, 98, 108, 101,
                                                        95, 97, 115, 115, 101, 116
                                                    ]
                                                ),
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 100,
                                                        101, 114, 105, 118, 101, 100, 95,
                                                        98, 97, 108, 97, 110, 99, 101
                                                    ]
                                                )
                                            ),
                                            option::borrow<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                        )
                                            && (
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) == @0xa
                                                    && aborts_of<error::permission_denied>(
                                                    31)
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        result_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 112, 111, 115, 105, 116
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(deposit_function)
                        )
                            && (
                                option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        result_of<function_info::check_dispatch_type_compatibility>(
                                            function_info::new_function_info_from_address(
                                                @0x1,
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 102,
                                                        117, 110, 103, 105, 98, 108, 101,
                                                        95, 97, 115, 115, 101, 116
                                                    ]
                                                ),
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 100,
                                                        101, 114, 105, 118, 101, 100, 95,
                                                        98, 97, 108, 97, 110, 99, 101
                                                    ]
                                                )
                                            ),
                                            option::borrow<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                        )
                                            && (
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) != @0xa
                                                    && object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                option::is_some<function_info::FunctionInfo>(deposit_function)
                    && (
                        result_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 112, 111, 115, 105, 116
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(deposit_function)
                        )
                            && (
                                option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        result_of<function_info::check_dispatch_type_compatibility>(
                                            function_info::new_function_info_from_address(
                                                @0x1,
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 102,
                                                        117, 110, 103, 105, 98, 108, 101,
                                                        95, 97, 115, 115, 101, 116
                                                    ]
                                                ),
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 100,
                                                        101, 114, 105, 118, 101, 100, 95,
                                                        98, 97, 108, 97, 110, 99, 101
                                                    ]
                                                )
                                            ),
                                            option::borrow<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                        )
                                            && (
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) != @0xa
                                                    && (
                                                        object::can_generate_delete_ref(
                                                            constructor_ref
                                                        )
                                                            && aborts_of<error::invalid_argument>(
                                                                18
                                                            )
                                                    )
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && !exists<Metadata>(
                                                            object::address_from_constructor_ref(
                                                                constructor_ref
                                                            )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && (
                                                            !exists<Metadata>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && aborts_of<error::not_found>(
                                                                    30
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S4 |~ exists<DispatchFunctionStore>(
                    signer::address_of(
                        ..S4 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            !option::is_some<function_info::FunctionInfo>(withdraw_function)
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            result_of<function_info::check_dispatch_type_compatibility>(
                                                function_info::new_function_info_from_address(
                                                    @0x1,
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            102, 117, 110, 103, 105, 98,
                                                            108, 101, 95, 97, 115, 115,
                                                            101, 116
                                                        ]
                                                    ),
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            100, 101, 114, 105, 118, 101,
                                                            100, 95, 98, 97, 108, 97, 110,
                                                            99, 101
                                                        ]
                                                    )
                                                ),
                                                option::borrow<function_info::FunctionInfo>(
                                                    derived_balance_function
                                                )
                                            )
                                                && (
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    ) != @0xa
                                                        && (
                                                            !object::can_generate_delete_ref(
                                                                constructor_ref
                                                            )
                                                                && (
                                                                    exists<Metadata>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && (
                                                                            !exists<
                                                                                DispatchFunctionStore>(
                                                                                object::address_from_constructor_ref(
                                                                                    constructor_ref
                                                                                )
                                                                            )
                                                                                && a
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && exists<Metadata>(
                                                            object::address_from_constructor_ref(
                                                                constructor_ref
                                                            )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && (
                                                            exists<Metadata>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && (
                                                                    exists<
                                                                        DispatchFunctionStore>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && aborts_of<error::already_exists>(
                                                                            29
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && aborts_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 114, 105, 118,
                                                101, 100, 95, 98, 97, 108, 97, 110, 99,
                                                101
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && aborts_of<function_info::new_function_info_from_address>(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 114, 105, 118, 101,
                                            100, 95, 98, 97, 108, 97, 110, 99, 101
                                        ]
                                    )
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(deposit_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && aborts_of<option::borrow<function_info::FunctionInfo>> (
                                    derived_balance_function
                                )
                        )
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && aborts_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                100, 101, 112, 111, 115, 105, 116
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(deposit_function)
                )
        );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                option::is_some<function_info::FunctionInfo>(deposit_function)
                    && aborts_of<function_info::new_function_info_from_address>(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                100, 101, 112, 111, 115, 105, 116
                            ]
                        )
                    )
            );
        aborts_if [inferred]!option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            option::is_some<function_info::FunctionInfo>(deposit_function)
                && aborts_of<option::borrow<function_info::FunctionInfo>> (
                    deposit_function
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && !result_of<function_info::check_dispatch_type_compatibility>(
            function_info::new_function_info_from_address(
                @0x1,
                string::utf8(
                    vector[
                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 102,
                        117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101, 116
                    ]
                ),
                string::utf8(
                    vector[
                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 119,
                        105, 116, 104, 100, 114, 97, 119
                    ]
                )
            ),
            option::borrow<function_info::FunctionInfo>(withdraw_function)
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                !result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && aborts_of<error::invalid_argument>(25)
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && object::address_from_constructor_ref(constructor_ref)
                                    == @0xa
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        !option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                !option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        ) == @0xa
                                            && aborts_of<error::permission_denied>(31)
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        !option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                !option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        ) != @0xa
                                            && object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        !option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                !option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        object::address_from_constructor_ref(
                                            constructor_ref
                                        ) != @0xa
                                            && (
                                                object::can_generate_delete_ref(
                                                    constructor_ref
                                                )
                                                    && aborts_of<error::invalid_argument>(
                                                    18)
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && !exists<Metadata>(
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && (
                                                    !exists<Metadata>(
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        )
                                                    )
                                                        && aborts_of<error::not_found>(30)
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S4 |~ exists<DispatchFunctionStore>(
                    signer::address_of(
                        ..S4 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            option::is_some<function_info::FunctionInfo>(withdraw_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 119, 105, 116, 104, 100, 114, 97, 119
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(withdraw_function)
                    )
                        && (
                            !option::is_some<function_info::FunctionInfo>(deposit_function)
                                && (
                                    !option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && (
                                                            exists<Metadata>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && (
                                                                    !exists<
                                                                        DispatchFunctionStore>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && a
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && exists<Metadata>(
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            !option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    object::address_from_constructor_ref(constructor_ref)
                                    != @0xa
                                        && (
                                            !object::can_generate_delete_ref(
                                                constructor_ref
                                            )
                                                && (
                                                    exists<Metadata>(
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        )
                                                    )
                                                        && (
                                                            exists<DispatchFunctionStore>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && aborts_of<error::already_exists>(
                                                                    29
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && !result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 114, 105, 118,
                                                101, 100, 95, 98, 97, 108, 97, 110, 99,
                                                101
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    !result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && aborts_of<error::invalid_argument>(27)
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && object::address_from_constructor_ref(
                                            constructor_ref
                                        ) == @0xa
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        !option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        result_of<function_info::check_dispatch_type_compatibility>(
                                            function_info::new_function_info_from_address(
                                                @0x1,
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 102,
                                                        117, 110, 103, 105, 98, 108, 101,
                                                        95, 97, 115, 115, 101, 116
                                                    ]
                                                ),
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 100,
                                                        101, 114, 105, 118, 101, 100, 95,
                                                        98, 97, 108, 97, 110, 99, 101
                                                    ]
                                                )
                                            ),
                                            option::borrow<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                        )
                                            && (
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) == @0xa
                                                    && aborts_of<error::permission_denied>(
                                                    31)
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        !option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        result_of<function_info::check_dispatch_type_compatibility>(
                                            function_info::new_function_info_from_address(
                                                @0x1,
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 102,
                                                        117, 110, 103, 105, 98, 108, 101,
                                                        95, 97, 115, 115, 101, 116
                                                    ]
                                                ),
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 100,
                                                        101, 114, 105, 118, 101, 100, 95,
                                                        98, 97, 108, 97, 110, 99, 101
                                                    ]
                                                )
                                            ),
                                            option::borrow<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                        )
                                            && (
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) != @0xa
                                                    && object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        !option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                option::is_some<function_info::FunctionInfo>(
                                    derived_balance_function
                                )
                                    && (
                                        result_of<function_info::check_dispatch_type_compatibility>(
                                            function_info::new_function_info_from_address(
                                                @0x1,
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 102,
                                                        117, 110, 103, 105, 98, 108, 101,
                                                        95, 97, 115, 115, 101, 116
                                                    ]
                                                ),
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 100,
                                                        101, 114, 105, 118, 101, 100, 95,
                                                        98, 97, 108, 97, 110, 99, 101
                                                    ]
                                                )
                                            ),
                                            option::borrow<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                        )
                                            && (
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) != @0xa
                                                    && (
                                                        object::can_generate_delete_ref(
                                                            constructor_ref
                                                        )
                                                            && aborts_of<error::invalid_argument>(
                                                                18
                                                            )
                                                    )
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && !exists<Metadata>(
                                                            object::address_from_constructor_ref(
                                                                constructor_ref
                                                            )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && (
                                                            !exists<Metadata>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && aborts_of<error::not_found>(
                                                                    30
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S4 |~ exists<DispatchFunctionStore>(
                    signer::address_of(
                        ..S4 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            option::is_some<function_info::FunctionInfo>(withdraw_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 119, 105, 116, 104, 100, 114, 97, 119
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(withdraw_function)
                    )
                        && (
                            !option::is_some<function_info::FunctionInfo>(deposit_function)
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            result_of<function_info::check_dispatch_type_compatibility>(
                                                function_info::new_function_info_from_address(
                                                    @0x1,
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            102, 117, 110, 103, 105, 98,
                                                            108, 101, 95, 97, 115, 115,
                                                            101, 116
                                                        ]
                                                    ),
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            100, 101, 114, 105, 118, 101,
                                                            100, 95, 98, 97, 108, 97, 110,
                                                            99, 101
                                                        ]
                                                    )
                                                ),
                                                option::borrow<function_info::FunctionInfo>(
                                                    derived_balance_function
                                                )
                                            )
                                                && (
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    ) != @0xa
                                                        && (
                                                            !object::can_generate_delete_ref(
                                                                constructor_ref
                                                            )
                                                                && (
                                                                    exists<Metadata>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && (
                                                                            !exists<
                                                                                DispatchFunctionStore>(
                                                                                object::address_from_constructor_ref(
                                                                                    constructor_ref
                                                                                )
                                                                            )
                                                                                && a
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && exists<Metadata>(
                                                            object::address_from_constructor_ref(
                                                                constructor_ref
                                                            )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && (
                                                            exists<Metadata>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && (
                                                                    exists<
                                                                        DispatchFunctionStore>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && aborts_of<error::already_exists>(
                                                                            29
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && aborts_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 114, 105, 118,
                                                101, 100, 95, 98, 97, 108, 97, 110, 99,
                                                101
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && aborts_of<function_info::new_function_info_from_address>(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 114, 105, 118, 101,
                                            100, 95, 98, 97, 108, 97, 110, 99, 101
                                        ]
                                    )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    !option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            option::is_some<function_info::FunctionInfo>(
                                derived_balance_function
                            )
                                && aborts_of<option::borrow<function_info::FunctionInfo>> (
                                    derived_balance_function
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && !result_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 112, 111, 115, 105, 116
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(deposit_function)
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            !result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && aborts_of<error::invalid_argument>(26)
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    !option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && object::address_from_constructor_ref(
                                            constructor_ref
                                        ) == @0xa
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 112, 111, 115,
                                                105, 116
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        deposit_function
                                    )
                                )
                                    && (
                                        !option::is_some<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                            && (
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) == @0xa
                                                    && aborts_of<error::permission_denied>(
                                                    31)
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 112, 111, 115,
                                                105, 116
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        deposit_function
                                    )
                                )
                                    && (
                                        !option::is_some<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                            && (
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) != @0xa
                                                    && object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 112, 111, 115,
                                                105, 116
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        deposit_function
                                    )
                                )
                                    && (
                                        !option::is_some<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                            && (
                                                object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) != @0xa
                                                    && (
                                                        object::can_generate_delete_ref(
                                                            constructor_ref
                                                        )
                                                            && aborts_of<error::invalid_argument>(
                                                                18
                                                            )
                                                    )
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    !option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && !exists<Metadata>(
                                                            object::address_from_constructor_ref(
                                                                constructor_ref
                                                            )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    !option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && (
                                                            !exists<Metadata>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && aborts_of<error::not_found>(
                                                                    30
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S4 |~ exists<DispatchFunctionStore>(
                    signer::address_of(
                        ..S4 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            option::is_some<function_info::FunctionInfo>(withdraw_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 119, 105, 116, 104, 100, 114, 97, 119
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(withdraw_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(deposit_function)
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 112,
                                                    111, 115, 105, 116
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            deposit_function
                                        )
                                    )
                                        && (
                                            !option::is_some<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                                && (
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    ) != @0xa
                                                        && (
                                                            !object::can_generate_delete_ref(
                                                                constructor_ref
                                                            )
                                                                && (
                                                                    exists<Metadata>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && (
                                                                            !exists<
                                                                                DispatchFunctionStore>(
                                                                                object::address_from_constructor_ref(
                                                                                    constructor_ref
                                                                                )
                                                                            )
                                                                                && a
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    !option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && exists<Metadata>(
                                                            object::address_from_constructor_ref(
                                                                constructor_ref
                                                            )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    !option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            object::address_from_constructor_ref(
                                                constructor_ref
                                            ) != @0xa
                                                && (
                                                    !object::can_generate_delete_ref(
                                                        constructor_ref
                                                    )
                                                        && (
                                                            exists<Metadata>(
                                                                object::address_from_constructor_ref(
                                                                    constructor_ref
                                                                )
                                                            )
                                                                && (
                                                                    exists<
                                                                        DispatchFunctionStore>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && aborts_of<error::already_exists>(
                                                                            29
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && !result_of<function_info::check_dispatch_type_compatibility>(
                                            function_info::new_function_info_from_address(
                                                @0x1,
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 102,
                                                        117, 110, 103, 105, 98, 108, 101,
                                                        95, 97, 115, 115, 101, 116
                                                    ]
                                                ),
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 100,
                                                        101, 114, 105, 118, 101, 100, 95,
                                                        98, 97, 108, 97, 110, 99, 101
                                                    ]
                                                )
                                            ),
                                            option::borrow<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            !result_of<function_info::check_dispatch_type_compatibility>(
                                                function_info::new_function_info_from_address(
                                                    @0x1,
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            102, 117, 110, 103, 105, 98,
                                                            108, 101, 95, 97, 115, 115,
                                                            101, 116
                                                        ]
                                                    ),
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            100, 101, 114, 105, 118, 101,
                                                            100, 95, 98, 97, 108, 97, 110,
                                                            99, 101
                                                        ]
                                                    )
                                                ),
                                                option::borrow<function_info::FunctionInfo>(
                                                    derived_balance_function
                                                )
                                            )
                                                && aborts_of<error::invalid_argument>(27)
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            result_of<function_info::check_dispatch_type_compatibility>(
                                                function_info::new_function_info_from_address(
                                                    @0x1,
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            102, 117, 110, 103, 105, 98,
                                                            108, 101, 95, 97, 115, 115,
                                                            101, 116
                                                        ]
                                                    ),
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            100, 101, 114, 105, 118, 101,
                                                            100, 95, 98, 97, 108, 97, 110,
                                                            99, 101
                                                        ]
                                                    )
                                                ),
                                                option::borrow<function_info::FunctionInfo>(
                                                    derived_balance_function
                                                )
                                            )
                                                && object::address_from_constructor_ref(
                                                    constructor_ref
                                                ) == @0xa
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 112, 111, 115,
                                                105, 116
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        deposit_function
                                    )
                                )
                                    && (
                                        option::is_some<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                            && (
                                                result_of<function_info::check_dispatch_type_compatibility>(
                                                    function_info::new_function_info_from_address(
                                                        @0x1,
                                                        string::utf8(
                                                            vector[
                                                                100, 105, 115, 112, 97,
                                                                116, 99, 104, 97, 98, 108,
                                                                101, 95, 102, 117, 110,
                                                                103, 105, 98, 108, 101, 95,
                                                                97, 115, 115, 101, 116
                                                            ]
                                                        ),
                                                        string::utf8(
                                                            vector[
                                                                100, 105, 115, 112, 97,
                                                                116, 99, 104, 97, 98, 108,
                                                                101, 95, 100, 101, 114,
                                                                105, 118, 101, 100, 95, 98,
                                                                97, 108, 97, 110, 99, 101
                                                            ]
                                                        )
                                                    ),
                                                    option::borrow<function_info::FunctionInfo>(
                                                        derived_balance_function
                                                    )
                                                )
                                                    && (
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        ) == @0xa
                                                            && aborts_of<error::permission_denied>(
                                                                31
                                                            )
                                                    )
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 112, 111, 115,
                                                105, 116
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        deposit_function
                                    )
                                )
                                    && (
                                        option::is_some<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                            && (
                                                result_of<function_info::check_dispatch_type_compatibility>(
                                                    function_info::new_function_info_from_address(
                                                        @0x1,
                                                        string::utf8(
                                                            vector[
                                                                100, 105, 115, 112, 97,
                                                                116, 99, 104, 97, 98, 108,
                                                                101, 95, 102, 117, 110,
                                                                103, 105, 98, 108, 101, 95,
                                                                97, 115, 115, 101, 116
                                                            ]
                                                        ),
                                                        string::utf8(
                                                            vector[
                                                                100, 105, 115, 112, 97,
                                                                116, 99, 104, 97, 98, 108,
                                                                101, 95, 100, 101, 114,
                                                                105, 118, 101, 100, 95, 98,
                                                                97, 108, 97, 110, 99, 101
                                                            ]
                                                        )
                                                    ),
                                                    option::borrow<function_info::FunctionInfo>(
                                                        derived_balance_function
                                                    )
                                                )
                                                    && (
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        ) != @0xa
                                                            && object::can_generate_delete_ref(
                                                                constructor_ref
                                                            )
                                                    )
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        )
            && (
                result_of<function_info::check_dispatch_type_compatibility>(
                    function_info::new_function_info_from_address(
                        @0x1,
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115,
                                101, 116
                            ]
                        ),
                        string::utf8(
                            vector[
                                100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                                119, 105, 116, 104, 100, 114, 97, 119
                            ]
                        )
                    ),
                    option::borrow<function_info::FunctionInfo>(withdraw_function)
                )
                    && (
                        option::is_some<function_info::FunctionInfo>(deposit_function)
                            && (
                                result_of<function_info::check_dispatch_type_compatibility>(
                                    function_info::new_function_info_from_address(
                                        @0x1,
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 102, 117, 110, 103, 105,
                                                98, 108, 101, 95, 97, 115, 115, 101, 116
                                            ]
                                        ),
                                        string::utf8(
                                            vector[
                                                100, 105, 115, 112, 97, 116, 99, 104, 97,
                                                98, 108, 101, 95, 100, 101, 112, 111, 115,
                                                105, 116
                                            ]
                                        )
                                    ),
                                    option::borrow<function_info::FunctionInfo>(
                                        deposit_function
                                    )
                                )
                                    && (
                                        option::is_some<function_info::FunctionInfo>(
                                            derived_balance_function
                                        )
                                            && (
                                                result_of<function_info::check_dispatch_type_compatibility>(
                                                    function_info::new_function_info_from_address(
                                                        @0x1,
                                                        string::utf8(
                                                            vector[
                                                                100, 105, 115, 112, 97,
                                                                116, 99, 104, 97, 98, 108,
                                                                101, 95, 102, 117, 110,
                                                                103, 105, 98, 108, 101, 95,
                                                                97, 115, 115, 101, 116
                                                            ]
                                                        ),
                                                        string::utf8(
                                                            vector[
                                                                100, 105, 115, 112, 97,
                                                                116, 99, 104, 97, 98, 108,
                                                                101, 95, 100, 101, 114,
                                                                105, 118, 101, 100, 95, 98,
                                                                97, 108, 97, 110, 99, 101
                                                            ]
                                                        )
                                                    ),
                                                    option::borrow<function_info::FunctionInfo>(
                                                        derived_balance_function
                                                    )
                                                )
                                                    && (
                                                        object::address_from_constructor_ref(
                                                            constructor_ref
                                                        ) != @0xa
                                                            && (
                                                                object::can_generate_delete_ref(
                                                                    constructor_ref
                                                                )
                                                                    && aborts_of<error::invalid_argument>(
                                                                        18
                                                                    )
                                                            )
                                                    )
                                            )
                                    )
                            )
                    )
            );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            result_of<function_info::check_dispatch_type_compatibility>(
                                                function_info::new_function_info_from_address(
                                                    @0x1,
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            102, 117, 110, 103, 105, 98,
                                                            108, 101, 95, 97, 115, 115,
                                                            101, 116
                                                        ]
                                                    ),
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            100, 101, 114, 105, 118, 101,
                                                            100, 95, 98, 97, 108, 97, 110,
                                                            99, 101
                                                        ]
                                                    )
                                                ),
                                                option::borrow<function_info::FunctionInfo>(
                                                    derived_balance_function
                                                )
                                            )
                                                && (
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    ) != @0xa
                                                        && (
                                                            !object::can_generate_delete_ref(
                                                                constructor_ref
                                                            )
                                                                && !exists<Metadata>(
                                                                    object::address_from_constructor_ref(
                                                                        constructor_ref
                                                                    )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            result_of<function_info::check_dispatch_type_compatibility>(
                                                function_info::new_function_info_from_address(
                                                    @0x1,
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            102, 117, 110, 103, 105, 98,
                                                            108, 101, 95, 97, 115, 115,
                                                            101, 116
                                                        ]
                                                    ),
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            100, 101, 114, 105, 118, 101,
                                                            100, 95, 98, 97, 108, 97, 110,
                                                            99, 101
                                                        ]
                                                    )
                                                ),
                                                option::borrow<function_info::FunctionInfo>(
                                                    derived_balance_function
                                                )
                                            )
                                                && (
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    ) != @0xa
                                                        && (
                                                            !object::can_generate_delete_ref(
                                                                constructor_ref
                                                            )
                                                                && (
                                                                    !exists<Metadata>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && aborts_of<error::not_found>(
                                                                            30
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred]({
            let a =
                S4 |~ exists<DispatchFunctionStore>(
                    signer::address_of(
                        ..S4 |~ result_of<object::generate_signer>(constructor_ref)
                    )
                );
            option::is_some<function_info::FunctionInfo>(withdraw_function)
                && (
                    result_of<function_info::check_dispatch_type_compatibility>(
                        function_info::new_function_info_from_address(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 119, 105, 116, 104, 100, 114, 97, 119
                                ]
                            )
                        ),
                        option::borrow<function_info::FunctionInfo>(withdraw_function)
                    )
                        && (
                            option::is_some<function_info::FunctionInfo>(deposit_function)
                                && (
                                    result_of<function_info::check_dispatch_type_compatibility>(
                                        function_info::new_function_info_from_address(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 112,
                                                    111, 115, 105, 116
                                                ]
                                            )
                                        ),
                                        option::borrow<function_info::FunctionInfo>(
                                            deposit_function
                                        )
                                    )
                                        && (
                                            option::is_some<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                                && (
                                                    result_of<function_info::check_dispatch_type_compatibility>(
                                                        function_info::new_function_info_from_address(
                                                            @0x1,
                                                            string::utf8(
                                                                vector[
                                                                    100, 105, 115, 112, 97,
                                                                    116, 99, 104, 97, 98,
                                                                    108, 101, 95, 102, 117,
                                                                    110, 103, 105, 98, 108,
                                                                    101, 95, 97, 115, 115,
                                                                    101, 116
                                                                ]
                                                            ),
                                                            string::utf8(
                                                                vector[
                                                                    100, 105, 115, 112, 97,
                                                                    116, 99, 104, 97, 98,
                                                                    108, 101, 95, 100, 101,
                                                                    114, 105, 118, 101,
                                                                    100, 95, 98, 97, 108,
                                                                    97, 110, 99, 101
                                                                ]
                                                            )
                                                        ),
                                                        option::borrow<function_info::FunctionInfo>(
                                                            derived_balance_function
                                                        )
                                                    )
                                                        && (
                                                            object::address_from_constructor_ref(
                                                                constructor_ref
                                                            ) != @0xa
                                                                && (
                                                                    !object::can_generate_delete_ref(
                                                                        constructor_ref
                                                                    )
                                                                        && (
                                                                            exists<Metadata>(
                                                                                object::address_from_constructor_ref(
                                                                                    constructor_ref
                                                                                )
                                                                            )
                                                                                && (
                                                                                    !exists<
                                                                                        DispatchFunctionStore>(
                                                                                        object::address_from_constructor_ref(
                                                                                            constructor_ref
                                                                                        )
                                                                                    )
                                                                                        && a
                                                                                )
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        });
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            result_of<function_info::check_dispatch_type_compatibility>(
                                                function_info::new_function_info_from_address(
                                                    @0x1,
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            102, 117, 110, 103, 105, 98,
                                                            108, 101, 95, 97, 115, 115,
                                                            101, 116
                                                        ]
                                                    ),
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            100, 101, 114, 105, 118, 101,
                                                            100, 95, 98, 97, 108, 97, 110,
                                                            99, 101
                                                        ]
                                                    )
                                                ),
                                                option::borrow<function_info::FunctionInfo>(
                                                    derived_balance_function
                                                )
                                            )
                                                && (
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    ) != @0xa
                                                        && (
                                                            !object::can_generate_delete_ref(
                                                                constructor_ref
                                                            )
                                                                && exists<Metadata>(
                                                                    object::address_from_constructor_ref(
                                                                        constructor_ref
                                                                    )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && (
                                            result_of<function_info::check_dispatch_type_compatibility>(
                                                function_info::new_function_info_from_address(
                                                    @0x1,
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            102, 117, 110, 103, 105, 98,
                                                            108, 101, 95, 97, 115, 115,
                                                            101, 116
                                                        ]
                                                    ),
                                                    string::utf8(
                                                        vector[
                                                            100, 105, 115, 112, 97, 116,
                                                            99, 104, 97, 98, 108, 101, 95,
                                                            100, 101, 114, 105, 118, 101,
                                                            100, 95, 98, 97, 108, 97, 110,
                                                            99, 101
                                                        ]
                                                    )
                                                ),
                                                option::borrow<function_info::FunctionInfo>(
                                                    derived_balance_function
                                                )
                                            )
                                                && (
                                                    object::address_from_constructor_ref(
                                                        constructor_ref
                                                    ) != @0xa
                                                        && (
                                                            !object::can_generate_delete_ref(
                                                                constructor_ref
                                                            )
                                                                && (
                                                                    exists<Metadata>(
                                                                        object::address_from_constructor_ref(
                                                                            constructor_ref
                                                                        )
                                                                    )
                                                                        && (
                                                                            exists<
                                                                                DispatchFunctionStore>(
                                                                                object::address_from_constructor_ref(
                                                                                    constructor_ref
                                                                                )
                                                                            )
                                                                                && aborts_of<error::already_exists>(
                                                                                    29
                                                                                )
                                                                        )
                                                                )
                                                        )
                                                )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && aborts_of<function_info::check_dispatch_type_compatibility>(
                                            function_info::new_function_info_from_address(
                                                @0x1,
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 102,
                                                        117, 110, 103, 105, 98, 108, 101,
                                                        95, 97, 115, 115, 101, 116
                                                    ]
                                                ),
                                                string::utf8(
                                                    vector[
                                                        100, 105, 115, 112, 97, 116, 99,
                                                        104, 97, 98, 108, 101, 95, 100,
                                                        101, 114, 105, 118, 101, 100, 95,
                                                        98, 97, 108, 97, 110, 99, 101
                                                    ]
                                                )
                                            ),
                                            option::borrow<function_info::FunctionInfo>(
                                                derived_balance_function
                                            )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && aborts_of<function_info::new_function_info_from_address>(
                                            @0x1,
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 102, 117, 110,
                                                    103, 105, 98, 108, 101, 95, 97, 115,
                                                    115, 101, 116
                                                ]
                                            ),
                                            string::utf8(
                                                vector[
                                                    100, 105, 115, 112, 97, 116, 99, 104,
                                                    97, 98, 108, 101, 95, 100, 101, 114,
                                                    105, 118, 101, 100, 95, 98, 97, 108,
                                                    97, 110, 99, 101
                                                ]
                                            )
                                        )
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && (
                            result_of<function_info::check_dispatch_type_compatibility>(
                                function_info::new_function_info_from_address(
                                    @0x1,
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 102, 117, 110, 103, 105, 98, 108,
                                            101, 95, 97, 115, 115, 101, 116
                                        ]
                                    ),
                                    string::utf8(
                                        vector[
                                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98,
                                            108, 101, 95, 100, 101, 112, 111, 115, 105,
                                            116
                                        ]
                                    )
                                ),
                                option::borrow<function_info::FunctionInfo>(
                                    deposit_function
                                )
                            )
                                && (
                                    option::is_some<function_info::FunctionInfo>(
                                        derived_balance_function
                                    )
                                        && aborts_of<option::borrow<function_info::FunctionInfo
                                        >> (derived_balance_function)
                                )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && aborts_of<function_info::check_dispatch_type_compatibility>(
                            function_info::new_function_info_from_address(
                                @0x1,
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 102, 117, 110, 103, 105, 98, 108, 101, 95,
                                        97, 115, 115, 101, 116
                                    ]
                                ),
                                string::utf8(
                                    vector[
                                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108,
                                        101, 95, 100, 101, 112, 111, 115, 105, 116
                                    ]
                                )
                            ),
                            option::borrow<function_info::FunctionInfo>(deposit_function)
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && aborts_of<function_info::new_function_info_from_address>(
                            @0x1,
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115,
                                    115, 101, 116
                                ]
                            ),
                            string::utf8(
                                vector[
                                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101,
                                    95, 100, 101, 112, 111, 115, 105, 116
                                ]
                            )
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && (
            result_of<function_info::check_dispatch_type_compatibility>(
                function_info::new_function_info_from_address(
                    @0x1,
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            102, 117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101,
                            116
                        ]
                    ),
                    string::utf8(
                        vector[
                            100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95,
                            119, 105, 116, 104, 100, 114, 97, 119
                        ]
                    )
                ),
                option::borrow<function_info::FunctionInfo>(withdraw_function)
            )
                && (
                    option::is_some<function_info::FunctionInfo>(deposit_function)
                        && aborts_of<option::borrow<function_info::FunctionInfo>> (
                            deposit_function
                        )
                )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && aborts_of<function_info::check_dispatch_type_compatibility>(
            function_info::new_function_info_from_address(
                @0x1,
                string::utf8(
                    vector[
                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 102,
                        117, 110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101, 116
                    ]
                ),
                string::utf8(
                    vector[
                        100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 119,
                        105, 116, 104, 100, 114, 97, 119
                    ]
                )
            ),
            option::borrow<function_info::FunctionInfo>(withdraw_function)
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && aborts_of<function_info::new_function_info_from_address>(
            @0x1,
            string::utf8(
                vector[
                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 102, 117,
                    110, 103, 105, 98, 108, 101, 95, 97, 115, 115, 101, 116
                ]
            ),
            string::utf8(
                vector[
                    100, 105, 115, 112, 97, 116, 99, 104, 97, 98, 108, 101, 95, 119, 105,
                    116, 104, 100, 114, 97, 119
                ]
            )
        );
        aborts_if [inferred] option::is_some<function_info::FunctionInfo>(
            withdraw_function
        ) && aborts_of<option::borrow<function_info::FunctionInfo>> (withdraw_function);
    }

    spec revoke_permission(
        _permissioned: &signer,
        _token_type: 0x1::object::Object<0x1::fungible_asset::Metadata>
    ) {
        pragma opaque = true;
        aborts_if [inferred] true;
    }

    spec set_frozen_flag<T: key>(
        self: &0x1::fungible_asset::TransferRef, store: 0x1::object::Object<T>, frozen: bool
    ) {
        pragma opaque = true;
        ensures [inferred](..S1 |~ self.metadata == result_of<store_metadata<T>> (store)) ==>
            (S1.. |~ ensures_of<set_frozen_flag_internal<T>> (store, frozen));
        aborts_if [inferred](..S1 |~ self.metadata == result_of<store_metadata<T>> (store))
            && (S1 |~ aborts_of<set_frozen_flag_internal<T>> (store, frozen));
        aborts_if [inferred]..S1 |~(self.metadata != result_of<store_metadata<T>> (store));
        aborts_if [inferred] aborts_of<store_metadata<T>> (store);
    }

    spec set_frozen_flag_internal<T: key>(
        store: 0x1::object::Object<T>, frozen: bool
    ) {
        use 0x1::event;
        use 0x1::object;
        pragma opaque = true;
        modifies FungibleStore[object::object_address<T>(store)];
        ensures [inferred] S1.. |~(
            ensures_of<event::emit<Frozen>> (
                Frozen {
                    store: object::object_address<T>(store),
                    frozen: frozen
                }
            )
        );
        ensures [inferred] {
            let a = object::object_address<T>(store);
            let b =
                update_field(
                    old(
                        FungibleStore[object::object_address<T>(store)]
                    ),
                    frozen,
                    frozen
                );
            ..S1 |~ update<FungibleStore>(a, b)
        };
        aborts_if [inferred] S1 |~(
            aborts_of<event::emit<Frozen>> (
                Frozen {
                    store: object::object_address<T>(store),
                    frozen: frozen
                }
            )
        );
        aborts_if [inferred]!exists<FungibleStore>(object::object_address<T>(store));
    }

    spec supply<T: key>(metadata: 0x1::object::Object<T>): 0x1::option::Option<u128> {
        use 0x1::object;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a = S1.. |~ result_of<supply_impl<T>> (metadata);
            let b =
                ..S1 |~ result_of<has_supply_dispatch_function>(
                    object::object_address<T>(metadata)
                );
            !b ==> result == a
        });
        aborts_if [inferred]..S1 |~(
            result_of<has_supply_dispatch_function>(object::object_address<T>(metadata))
        );
    }

    spec supply_impl<T: key>(metadata: 0x1::object::Object<T>): 0x1::option::Option<u128> {
        use 0x1::option;
        use 0x1::object;
        use 0x1::aggregator_v2;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard] exists<ConcurrentSupply>(
            object::object_address<T>(metadata)
        ) ==>
            result
                == option::some<u128>(
                    result_of<aggregator_v2::read<u128>> (
                        ConcurrentSupply[object::object_address<T>(metadata)].current
                    )
                );
        ensures [inferred]!exists<ConcurrentSupply>(object::object_address<T>(metadata)) ==>
            result == option::none<u128>();
    }

    spec supply_with_ref<T: key>(
        self: &0x1::fungible_asset::RawSupplyRef, metadata: 0x1::object::Object<T>
    ): 0x1::option::Option<u128> {
        use 0x1::object;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] object::object_address<Metadata>(self.metadata)
            == object::object_address<T>(metadata) ==>
            result == result_of<supply_impl<T>> (metadata);
        aborts_if [inferred] object::object_address<Metadata>(self.metadata)
            != object::object_address<T>(metadata);
    }

    spec unchecked_deposit_with_no_events(
        store_addr: address, fa: 0x1::fungible_asset::FungibleAsset
    ) {
        use 0x1::error;
        use 0x1::aggregator_v2;
        pragma opaque = true;
        modifies ConcurrentFungibleBalance[store_addr];
        ensures [inferred] old(exists<FungibleStore>(store_addr))
            && (
                fa.metadata == old(FungibleStore[store_addr]).metadata
                    && (fa.amount != 0
                        && old(FungibleStore[store_addr]).balance == 0)
            ) ==>
            ConcurrentFungibleBalance[store_addr].balance
                == ConcurrentFungibleBalance[store_addr].balance;
        ensures [inferred] old(exists<FungibleStore>(store_addr))
            && (
                fa.metadata == old(FungibleStore[store_addr]).metadata
                    && (fa.amount != 0
                        && old(FungibleStore[store_addr]).balance == 0)
            ) ==>
            ensures_of<aggregator_v2::add<u64>> (
                old(ConcurrentFungibleBalance[store_addr]).balance, fa.amount
            );
        ensures [inferred] old(exists<FungibleStore>(store_addr))
            && (
                fa.metadata == old(FungibleStore[store_addr]).metadata
                    && (fa.amount != 0
                        && old(FungibleStore[store_addr]).balance != 0)
            ) ==>
            update<FungibleStore>(
                store_addr,
                update_field(
                    old(FungibleStore[store_addr]),
                    balance,
                    old(FungibleStore[store_addr]).balance + fa.amount
                )
            );
        aborts_if [inferred]!exists<FungibleStore>(store_addr);
        aborts_if [inferred] exists<FungibleStore>(store_addr)
            && fa.metadata != FungibleStore[store_addr].metadata;
        aborts_if [inferred] exists<FungibleStore>(store_addr)
            && (
                fa.metadata != FungibleStore[store_addr].metadata
                    && aborts_of<error::invalid_argument>(11)
            );
        aborts_if [inferred] exists<FungibleStore>(store_addr)
            && (
                fa.metadata == FungibleStore[store_addr].metadata
                    && (
                        fa.amount != 0
                            && (
                                FungibleStore[store_addr].balance != 0
                                    && FungibleStore[store_addr].balance + fa.amount
                                        > MAX_U64
                            )
                    )
            );
    }

    spec upgrade_store_to_concurrent<T: key>(
        owner: &signer, store: 0x1::object::Object<T>
    ) {
        use 0x1::signer;
        use 0x1::error;
        use 0x1::features;
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred]!exists<ConcurrentFungibleBalance>(
            object::object_address<T>(store)
        )
            && (
                (..S1 |~ result_of<object::owns<T>> (store, signer::address_of(owner)))
                    && (
                        (S1..S2 |~ !result_of<is_frozen<T>> (store))
                            && (
                                S2..S3 |~ result_of<features::concurrent_fungible_balance_enabled>()
                            )
                    )
            ) ==>
            (
                S3.. |~ ensures_of<ensure_store_upgraded_to_concurrent_internal>(
                    object::object_address<T>(store)
                )
            );
        aborts_if [inferred]({
            let a = ..S1 |~ result_of<object::owns<T>> (store, signer::address_of(owner));
            !exists<ConcurrentFungibleBalance>(object::object_address<T>(store)) && !a
        });
        aborts_if [inferred]({
            let a = ..S1 |~ result_of<object::owns<T>> (store, signer::address_of(owner));
            !exists<ConcurrentFungibleBalance>(object::object_address<T>(store))
                && (!a
                    && aborts_of<error::permission_denied>(8))
        });
        aborts_if [inferred]({
            let a = S1..S2 |~ result_of<is_frozen<T>> (store);
            let b = ..S1 |~ result_of<object::owns<T>> (store, signer::address_of(owner));
            !exists<ConcurrentFungibleBalance>(object::object_address<T>(store))
                && (b && a)
        });
        aborts_if [inferred]({
            let a = S1..S2 |~ result_of<is_frozen<T>> (store);
            let b = ..S1 |~ result_of<object::owns<T>> (store, signer::address_of(owner));
            !exists<ConcurrentFungibleBalance>(object::object_address<T>(store))
                && (b
                    && (a
                        && aborts_of<error::invalid_argument>(3)))
        });
        aborts_if [inferred]({
            let a = S2..S3 |~ result_of<features::concurrent_fungible_balance_enabled>();
            let b = S1..S2 |~ result_of<is_frozen<T>> (store);
            let c =
                S3 |~ aborts_of<ensure_store_upgraded_to_concurrent_internal>(
                    object::object_address<T>(store)
                );
            let d = ..S1 |~ result_of<object::owns<T>> (store, signer::address_of(owner));
            !exists<ConcurrentFungibleBalance>(object::object_address<T>(store))
                && (d && (!b && (a && c)))
        });
        aborts_if [inferred]({
            let a = S2..S3 |~ result_of<features::concurrent_fungible_balance_enabled>();
            let b = S1..S2 |~ result_of<is_frozen<T>> (store);
            let c = ..S1 |~ result_of<object::owns<T>> (store, signer::address_of(owner));
            !exists<ConcurrentFungibleBalance>(object::object_address<T>(store))
                && (c && (!b && !a))
        });
        aborts_if [inferred]({
            let a = S2..S3 |~ result_of<features::concurrent_fungible_balance_enabled>();
            let b = S1..S2 |~ result_of<is_frozen<T>> (store);
            let c = ..S1 |~ result_of<object::owns<T>> (store, signer::address_of(owner));
            !exists<ConcurrentFungibleBalance>(object::object_address<T>(store))
                && (c && (!b && (!a && aborts_of<error::invalid_argument>(32))))
        });
        aborts_if [inferred]({
            let a = S2 |~ aborts_of<features::concurrent_fungible_balance_enabled>();
            let b = S1..S2 |~ result_of<is_frozen<T>> (store);
            let c = ..S1 |~ result_of<object::owns<T>> (store, signer::address_of(owner));
            !exists<ConcurrentFungibleBalance>(object::object_address<T>(store))
                && (c && (!b && a))
        });
    }

    spec upgrade_to_concurrent(ref: &0x1::object::ExtendRef) {
        use 0x1::signer;
        use 0x1::features;
        use 0x1::object;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies ConcurrentSupply[
            signer::address_of(object::spec_generate_signer_for_extending(ref))
        ];
        modifies Supply[object::address_from_extend_ref(ref)];
        aborts_if [inferred] aborts_of<features::concurrent_fungible_assets_enabled>();
    }

    spec withdraw<T: key>(
        owner: &signer, store: 0x1::object::Object<T>, amount: u64
    ): 0x1::fungible_asset::FungibleAsset {
        use 0x1::object;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] result
            == (
                S1.. |~ result_of<unchecked_withdraw>(
                    object::object_address<T>(store), amount
                )
            );
        ensures [inferred]..S1 |~(ensures_of<withdraw_sanity_check<T>> (owner, store, true));
    }

    spec withdraw_dispatch_function<T: key>(
        store: 0x1::object::Object<T>
    ): 0x1::option::Option<0x1::function_info::FunctionInfo> {
        use 0x1::option;
        use 0x1::object;
        use 0x1::function_info;
        pragma opaque = true;
        ensures [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && exists<DispatchFunctionStore>(
                object::object_address<Metadata>(
                    FungibleStore[object::object_address<T>(store)].metadata
                )
            ) ==>
            result
                == DispatchFunctionStore[
                    object::object_address<Metadata>(
                        FungibleStore[object::object_address<T>(store)].metadata
                    )
                ].withdraw_function;
        ensures [inferred] exists<FungibleStore>(object::object_address<T>(store))
            && !exists<DispatchFunctionStore>(
                object::object_address<Metadata>(
                    FungibleStore[object::object_address<T>(store)].metadata
                )
            ) ==>
            result == option::none<function_info::FunctionInfo>();
        aborts_if [inferred]!exists<FungibleStore>(object::object_address<T>(store));
    }

    spec withdraw_sanity_check<T: key>(
        owner: &signer, store: 0x1::object::Object<T>, abort_on_dispatch: bool
    ) {
        use 0x1::signer;
        use 0x1::object;
        pragma opaque;
        pragma aborts_if_is_partial = false;
        let store_addr = object::object_address<T>(store);
        aborts_if aborts_of<object::owns<T>> (store, signer::address_of(owner));
        aborts_if !aborts_of<object::owns<T>> (store, signer::address_of(owner))
            && !result_of<object::owns<T>> (store, signer::address_of(owner));
        aborts_if !aborts_of<object::owns<T>> (store, signer::address_of(owner))
            && result_of<object::owns<T>> (store, signer::address_of(owner))
            && !exists<FungibleStore>(store_addr);
        aborts_if !aborts_of<object::owns<T>> (store, signer::address_of(owner))
            && result_of<object::owns<T>> (store, signer::address_of(owner))
            && exists<FungibleStore>(store_addr)
            && abort_on_dispatch
            && result_of<has_withdraw_dispatch_function>(
                FungibleStore[store_addr].metadata
            );
        aborts_if !aborts_of<object::owns<T>> (store, signer::address_of(owner))
            && result_of<object::owns<T>> (store, signer::address_of(owner))
            && exists<FungibleStore>(store_addr) && FungibleStore[store_addr].frozen;
    }

    spec withdraw_with_ref<T: key>(
        self: &0x1::fungible_asset::TransferRef, store: 0x1::object::Object<T>, amount: u64
    ): 0x1::fungible_asset::FungibleAsset {
        use 0x1::object;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a = ..S1 |~ result_of<store_metadata<T>> (store);
            let b =
                S1.. |~ result_of<unchecked_withdraw>(
                    object::object_address<T>(store), amount
                );
            self.metadata == a ==> result == b
        });
        aborts_if [inferred]..S1 |~(self.metadata != result_of<store_metadata<T>> (store));
        aborts_if [inferred] aborts_of<store_metadata<T>> (store);
    }

    spec zero<T: key>(metadata: 0x1::object::Object<T>): 0x1::fungible_asset::FungibleAsset {
        use 0x1::object;
        pragma opaque = true;
        ensures [inferred] result
            == FungibleAsset {
                metadata: object::Object<Metadata> { inner: metadata.inner },
                amount: 0
            };
        aborts_if [inferred] aborts_of<object::convert<T, Metadata>> (metadata);
    }
}
