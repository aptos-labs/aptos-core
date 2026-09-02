spec aptos_framework::primary_fungible_store {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Creating a fungible asset with primary store support should initiate a derived reference and store it
    /// under the metadata object.
    /// Criticality: Medium
    /// Implementation: The function create_primary_store_enabled_fungible_asset makes an existing object, fungible, via
    /// the fungible_asset::add_fungibility function and initializes the DeriveRefPod resource by generating a DeriveRef
    /// for the object and then stores it under the object address.
    /// Enforcement: Audited that the DeriveRefPod has been properly initialized and stored under the metadata object.
    ///
    /// No.: 2
    /// Requirement: Fetching and creating a primary fungible store of an asset should only succeed if the object supports
    /// primary store.
    /// Criticality: Low
    /// Implementation: The function create_primary_store is used to create a primary store by borrowing the DeriveRef
    /// resource from the object. In case the resource does not exist, creation will fail. The function
    /// ensure_primary_store_exists is used to fetch the primary store if it exists, otherwise it will create one via
    /// the create_primary function.
    /// Enforcement: Audited that it aborts if the DeriveRefPod doesn't exist. Audited that it aborts if the
    /// FungibleStore resource exists already under the object address.
    ///
    /// No.: 3
    /// Requirement: It should be possible to create a primary store to hold a fungible asset.
    /// Criticality: Medium
    /// Implementation: The function create_primary_store borrows the DeriveRef resource from DeriveRefPod and then
    /// creates the store which is returned.
    /// Enforcement: Audited that it returns the newly created FungibleStore.
    ///
    /// No.: 4
    /// Requirement: Fetching the balance or the frozen status of a primary store should never abort.
    /// Criticality: Low
    /// Implementation: The function balance returns the balance of the store, if the store exists, otherwise it returns 0.
    /// The function is_frozen returns the frozen flag of the fungible store, if the store exists, otherwise it returns
    /// false.
    /// Enforcement: Audited that the balance function returns the balance of the FungibleStore. Audited that the
    /// is_frozen function returns the frozen status of the FungibleStore resource. Audited that it never aborts.
    ///
    /// No.: 5
    /// Requirement: The ability to withdraw, deposit, transfer, mint and burn should only be available for assets with
    /// primary store support.
    /// Criticality: Medium
    /// Implementation: The primary store is fetched before performing either of withdraw, deposit, transfer, mint, burn
    /// operation. If the FungibleStore resource doesn't exist the operation will fail.
    /// Enforcement: Audited that it aborts if the primary store FungibleStore doesn't exist.
    ///
    /// No.: 6
    /// Requirement: The action of depositing a fungible asset of the same type as the store should never fail if the store
    /// is not frozen.
    /// Criticality: Medium
    /// Implementation: The function deposit fetches the owner's store, if it doesn't exist it will be created, and then
    /// deposits the fungible asset to it. The function deposit_with_ref fetches the owner's store, if it doesn't exist
    /// it will be created, and then deposit the fungible asset via the fungible_asset::deposit_with_ref function.
    /// Depositing fails if the metadata of the FungibleStore and FungibleAsset differs.
    /// Enforcement: Audited that it aborts if the store is frozen (deposit). Audited that the balance of the store is
    /// increased by the deposit amount (deposit, deposit_with_ref). Audited that it aborts if the metadata of the store
    /// and the asset differs (deposit, deposit_with_ref).
    ///
    /// No.: 7
    /// Requirement: Withdrawing should only be allowed to the owner of an existing store with sufficient balance.
    /// Criticality: Critical
    /// Implementation: The withdraw function fetches the owner's store via the primary_store function and then calls
    /// fungible_asset::withdraw which validates the owner of the store, checks the frozen status and the balance of the
    /// store. The withdraw_with_ref function fetches the store of the owner via primary_store function and calls the
    /// fungible_asset::withdraw_with_ref which validates transfer_ref's metadata with the withdrawing stores metadata,
    /// and the balance of the store.
    /// Enforcement: Audited that it aborts if the owner doesn't own the store (withdraw). Audited that it aborts if the
    /// store is frozen (withdraw). Audited that it aborts if the transfer ref's metadata doesn't match the withdrawing
    /// store's metadata (withdraw_with_ref). Audited that it aborts if the store doesn't have sufficient balance.
    /// Audited that the store is not burned. Audited that the balance of the store is decreased by the amount withdrawn.
    ///
    /// No.: 8
    /// Requirement: Only the fungible store owner is allowed to unburn a burned store.
    /// Criticality: High
    /// Implementation: The function may_be_unburn checks if the store is burned and then proceeds to call
    /// object::unburn which ensures that the owner of the object matches the address of the signer.
    /// Enforcement: Audited that the store is unburned successfully.
    ///
    /// No.: 9
    /// Requirement: Only the owner of a primary store can transfer its balance to any recipient's primary store.
    /// Criticality: High
    /// Implementation: The function transfer fetches sender and recipient's primary stores, if the sender's store is
    /// burned it unburns the store and calls the fungile_asset::transfer to proceed with the transfer, which first
    /// withdraws the assets from the sender's store and then deposits to the recipient's store.
    /// The function transfer_with_ref fetches the sender's and recipient's stores and calls the
    /// fungible_asset::transfer_with_ref function which withdraws the asset with the ref from the sender and deposits
    /// the asset to the recipient with the ref.
    /// Enforcement: Audited the deposit and withdraw (transfer). Audited the deposit_with_ref and
    /// withdraw_with_ref (transfer_with_ref). Audited that the store balance of the sender is decreased by the
    /// specified amount and its added to the recipients store. (transfer, transfer_with_ref) Audited that the sender's
    /// store is not burned (transfer).
    ///
    /// No.: 10
    /// Requirement: Minting an amount of assets to an unfrozen store is only allowed with a valid mint reference.
    /// Criticality: High
    /// Implementation: The mint function fetches the primary store and calls the fungible_asset::mint_to, which mints
    /// with MintRef's metadata which internally validates the amount and the increases the total supply of the asset.
    /// And the minted asset is deposited to the provided store by validating that the store is unfrozen and the store's
    /// metadata is the same as the depositing asset's metadata.
    /// Enforcement: Audited that it aborts if the amount is equal to 0. Audited that it aborts if the store is frozen.
    /// Audited that it aborts if the mint_ref's metadata is not the same as the store's metadata. Audited that the
    /// asset's total supply is increased by the amount minted. Audited that the balance of the store is increased by
    /// the minted amount.
    ///
    /// No.: 11
    /// Requirement: Burning an amount of assets from an existing unfrozen store is only allowed with a valid burn
    /// reference.
    /// Criticality: High
    /// Implementation: The burn function fetches the primary store and calls the fungible_asset::burn_from function
    /// which withdraws the amount from the store while enforcing that the store has enough balance and burns the
    /// withdrawn asset after validating the asset's metadata and the BurnRef's metadata followed by decreasing the
    /// supply of the asset.
    /// Enforcement: Audited that it aborts if the metadata of the store is not same as the BurnRef's metadata.
    /// Audited that it aborts if the burning amount is 0. Audited that it aborts if the store doesn't have enough
    /// balance. Audited that it aborts if the asset's metadata is not same as the BurnRef's metadata. Audited that the
    /// total supply of the asset is decreased. Audited that the store's balance is reduced by the amount burned.
    ///
    /// No.: 12
    /// Requirement: Setting the frozen flag of a store is only allowed with a valid reference.
    /// Criticality: High
    /// Implementation: The function set_frozen_flag fetches the primary store and calls fungible_asset::set_frozen_flag
    /// which validates the TransferRef's metadata with the store's metadata and then updates the frozen flag.
    /// Enforcement: Audited that it aborts if the store's metadata is not same as the TransferRef's metadata.
    /// Audited that the status of the frozen flag is updated correctly.
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
    }

    spec fun spec_primary_store_exists<T: key>(
        account: address, metadata: Object<T>
    ): bool {
        fungible_asset::store_exists(spec_primary_store_address(account, metadata))
    }

    spec fun spec_primary_store_address<T: key>(
        owner: address, metadata: Object<T>
    ): address {
        let metadata_addr = object::object_address(metadata);
        object::spec_create_user_derived_object_address(owner, metadata_addr)
    }

    spec primary_store_address<T: key>(
        owner: address, metadata: Object<T>
    ): address {
        pragma opaque;
        aborts_if false;
        ensures result == spec_primary_store_address(owner, metadata);
    }

    spec primary_store_exists<T: key>(
        account: address, metadata: Object<T>
    ): bool {
        use 0x1::object;
        use 0x1::fungible_asset;
        pragma opaque;
        aborts_if false;
        ensures result == spec_primary_store_exists(account, metadata);
        ensures [inferred] result_of<fungible_asset::store_exists>({
            let metadata_addr = object::object_address<T>(metadata);
            object::spec_create_user_derived_object_address(account, metadata_addr)
        }) == fungible_asset::store_exists({
            let metadata_addr = object::object_address<T>(metadata);
            object::spec_create_user_derived_object_address(account, metadata_addr)
        }) ==>
            result
                == result_of<fungible_asset::store_exists>({
                    let metadata_addr = object::object_address<T>(metadata);
                    object::spec_create_user_derived_object_address(account, metadata_addr)
                });
        aborts_if [inferred] false;
    }

    spec primary_store<T: key>(owner: address, metadata: Object<T>): Object<FungibleStore> {
        use 0x1::object;
        use 0x1::fungible_asset;
        pragma opaque;
        let store_addr = spec_primary_store_address(owner, metadata);
        aborts_if !exists<object::ObjectCore>(store_addr);
        aborts_if !object::spec_exists_at<FungibleStore>(store_addr);
        ensures object::object_address(result) == store_addr;
        ensures [inferred] object::object_address<fungible_asset::FungibleStore>(
            object::address_to_object<fungible_asset::FungibleStore>({
                let metadata_addr = object::object_address<T>(metadata);
                object::spec_create_user_derived_object_address(owner, metadata_addr)
            })
        ) == {
            let metadata_addr = object::object_address<T>(metadata);
            object::spec_create_user_derived_object_address(owner, metadata_addr)
        } ==>
            result
                == object::address_to_object<fungible_asset::FungibleStore>({
                    let metadata_addr = object::object_address<T>(metadata);
                    object::spec_create_user_derived_object_address(
                        owner, metadata_addr
                    )
                });
        aborts_if [inferred] aborts_of<object::address_to_object<fungible_asset::FungibleStore
            >> ({
            let metadata_addr = object::object_address<T>(metadata);
            object::spec_create_user_derived_object_address(owner, metadata_addr)
        });
    }

    spec burn(
        burn_ref: &0x1::fungible_asset::BurnRef, owner: address, amount: u64
    ) {
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<primary_store<fungible_asset::Metadata>> (
                    owner, fungible_asset::burn_ref_metadata(burn_ref)
                );
            S1.. |~ ensures_of<fungible_asset::burn_from<fungible_asset::FungibleStore>> (
                burn_ref, a, amount
            )
        });
        aborts_if [inferred] aborts_of<primary_store<fungible_asset::Metadata>> (
            owner, fungible_asset::burn_ref_metadata(burn_ref)
        );
    }

    spec transfer<T: key>(
        sender: &signer,
        metadata: 0x1::object::Object<T>,
        recipient: address,
        amount: u64
    ) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        aborts_if [inferred = sathard] aborts_of<ensure_primary_store_exists<T>> (
            signer::address_of(sender), metadata
        );
    }

    spec transfer_with_ref(
        transfer_ref: &0x1::fungible_asset::TransferRef,
        from: address,
        to: address,
        amount: u64
    ) {
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<primary_store<fungible_asset::Metadata>> (
                    from, fungible_asset::transfer_ref_metadata(transfer_ref)
                );
            let b =
                S1..S2 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    to, fungible_asset::transfer_ref_metadata(transfer_ref)
                );
            S2.. |~ ensures_of<fungible_asset::transfer_with_ref<fungible_asset::FungibleStore
                >> (transfer_ref, a, b, amount)
        });
        aborts_if [inferred] S1 |~(
            aborts_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                to, fungible_asset::transfer_ref_metadata(transfer_ref)
            )
        );
        aborts_if [inferred] aborts_of<primary_store<fungible_asset::Metadata>> (
            from, fungible_asset::transfer_ref_metadata(transfer_ref)
        );
    }

    spec balance<T: key>(
        account: address, metadata: 0x1::object::Object<T>
    ): u64 {
        use 0x1::fungible_asset;
        use 0x1::dispatchable_fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard](
            ..S1 |~ result_of<primary_store_exists<T>> (account, metadata)
        ) ==>
            result
                == {
                    let a = S1..S2 |~ result_of<primary_store<T>> (account, metadata);
                    S2.. |~ result_of<dispatchable_fungible_asset::derived_balance<fungible_asset::FungibleStore
                        >> (a)
                };
        ensures [inferred = sathard]({
            let a = ..S1 |~ result_of<primary_store_exists<T>> (account, metadata);
            !a ==> result == 0
        });
        aborts_if [inferred = sathard](
            ..S1 |~ result_of<primary_store_exists<T>> (account, metadata)
        ) && (S1 |~ aborts_of<primary_store<T>> (account, metadata));
    }

    spec deposit(owner: address, fa: 0x1::fungible_asset::FungibleAsset) {
        use 0x1::object;
        use 0x1::fungible_asset;
        use 0x1::dispatchable_fungible_asset;
        let metadata = fungible_asset::asset_metadata(fa);
        let metadata_addr = object::object_address<fungible_asset::Metadata>(metadata);
        let derive_from = DeriveRefPod[metadata_addr].metadata_derive_ref.self;
        let store_addr = object::spec_create_user_derived_object_address(
            owner, derive_from
        );
        modifies object::ObjectCore[store_addr];
        modifies object::Untransferable[store_addr];
        modifies fungible_asset::FungibleStore[store_addr];
        modifies fungible_asset::ConcurrentFungibleBalance[store_addr];
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if aborts_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
            owner, metadata
        );
        aborts_if ({
            let store_after_creation =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    owner, metadata
                );
            !aborts_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                owner, metadata
            ) && (
                S1 |~ aborts_of<dispatchable_fungible_asset::deposit<fungible_asset::FungibleStore
                >> (store_after_creation, fa)
            )
        });
        ensures ({
            let store_after_creation =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    owner, metadata
                );
            !aborts_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                owner, metadata
            ) && !(
                S1 |~ aborts_of<dispatchable_fungible_asset::deposit<fungible_asset::FungibleStore
                >> (store_after_creation, fa)
            ) ==>
                (
                    S1.. |~ ensures_of<dispatchable_fungible_asset::deposit<fungible_asset::FungibleStore
                    >> (store_after_creation, fa)
                )
        });
    }

    spec deposit_with_ref(
        transfer_ref: &0x1::fungible_asset::TransferRef,
        owner: address,
        fa: 0x1::fungible_asset::FungibleAsset
    ) {
        use 0x1::fungible_asset;
        pragma opaque = true;
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    owner, fungible_asset::transfer_ref_metadata(transfer_ref)
                );
            S1.. |~ ensures_of<fungible_asset::deposit_with_ref<fungible_asset::FungibleStore
                >> (transfer_ref, a, fa)
        });
        aborts_if [inferred]({
            let a =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    owner, fungible_asset::transfer_ref_metadata(transfer_ref)
                );
            S1 |~ aborts_of<fungible_asset::deposit_with_ref<fungible_asset::FungibleStore
                >> (transfer_ref, a, fa)
        });
        aborts_if [inferred] aborts_of<ensure_primary_store_exists<fungible_asset::Metadata
            >> (owner, fungible_asset::transfer_ref_metadata(transfer_ref));
    }

    spec is_balance_at_least<T: key>(
        account: address, metadata: 0x1::object::Object<T>, amount: u64
    ): bool {
        use 0x1::fungible_asset;
        use 0x1::dispatchable_fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard](
            ..S1 |~ result_of<primary_store_exists<T>> (account, metadata)
        ) ==>
            result
                == {
                    let a = S1..S2 |~ result_of<primary_store<T>> (account, metadata);
                    S2.. |~ result_of<dispatchable_fungible_asset::is_derived_balance_at_least<fungible_asset::FungibleStore
                        >> (a, amount)
                };
        ensures [inferred = sathard]({
            let a = ..S1 |~ result_of<primary_store_exists<T>> (account, metadata);
            !a ==> result == (amount == 0)
        });
        aborts_if [inferred = sathard](
            ..S1 |~ result_of<primary_store_exists<T>> (account, metadata)
        ) && (S1 |~ aborts_of<primary_store<T>> (account, metadata));
    }

    spec is_frozen<T: key>(
        account: address, metadata: 0x1::object::Object<T>
    ): bool {
        use 0x1::fungible_asset;
        pragma opaque = true;
        ensures [inferred](..S1 |~ result_of<primary_store_exists<T>> (account, metadata)) ==>
            result
                == {
                    let a = S1..S2 |~ result_of<primary_store<T>> (account, metadata);
                    S2.. |~ result_of<fungible_asset::is_frozen<fungible_asset::FungibleStore
                        >> (a)
                };
        ensures [inferred]({
            let a = ..S1 |~ result_of<primary_store_exists<T>> (account, metadata);
            !a ==> result == false
        });
        aborts_if [inferred](..S1 |~ result_of<primary_store_exists<T>> (account, metadata))
            && (S1 |~ aborts_of<primary_store<T>> (account, metadata));
    }

    spec mint(
        mint_ref: &0x1::fungible_asset::MintRef, owner: address, amount: u64
    ) {
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    owner, fungible_asset::mint_ref_metadata(mint_ref)
                );
            S1.. |~ ensures_of<fungible_asset::mint_to<fungible_asset::FungibleStore>> (
                mint_ref, a, amount
            )
        });
        aborts_if [inferred] aborts_of<ensure_primary_store_exists<fungible_asset::Metadata
            >> (owner, fungible_asset::mint_ref_metadata(mint_ref));
    }

    spec set_frozen_flag(
        transfer_ref: &0x1::fungible_asset::TransferRef, owner: address, frozen: bool
    ) {
        use 0x1::fungible_asset;
        pragma opaque = true;
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    owner, fungible_asset::transfer_ref_metadata(transfer_ref)
                );
            S1.. |~ ensures_of<fungible_asset::set_frozen_flag<fungible_asset::FungibleStore
                >> (transfer_ref, a, frozen)
        });
        aborts_if [inferred]({
            let a =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    owner, fungible_asset::transfer_ref_metadata(transfer_ref)
                );
            S1 |~ aborts_of<fungible_asset::set_frozen_flag<fungible_asset::FungibleStore
                >> (transfer_ref, a, frozen)
        });
        aborts_if [inferred] aborts_of<ensure_primary_store_exists<fungible_asset::Metadata
            >> (owner, fungible_asset::transfer_ref_metadata(transfer_ref));
    }

    spec upgrade_to_concurrent(
        owner: &signer, metadata: 0x1::object::Object<0x1::fungible_asset::Metadata>
    ) {
        use 0x1::signer;
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    signer::address_of(owner), metadata
                );
            S1.. |~ ensures_of<fungible_asset::upgrade_store_to_concurrent<fungible_asset::FungibleStore
                >> (owner, a)
        });
        aborts_if [inferred]({
            let a =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    signer::address_of(owner), metadata
                );
            S1 |~ aborts_of<fungible_asset::upgrade_store_to_concurrent<fungible_asset::FungibleStore
                >> (owner, a)
        });
        aborts_if [inferred] aborts_of<ensure_primary_store_exists<fungible_asset::Metadata
            >> (signer::address_of(owner), metadata);
    }

    spec withdraw<T: key>(
        owner: &signer, metadata: 0x1::object::Object<T>, amount: u64
    ): 0x1::fungible_asset::FungibleAsset {
        use 0x1::signer;
        use 0x1::object;
        use 0x1::fungible_asset;
        use 0x1::dispatchable_fungible_asset;
        let owner_addr = signer::address_of(owner);
        let metadata_addr = object::object_address<T>(metadata);
        let derive_from = DeriveRefPod[metadata_addr].metadata_derive_ref.self;
        let store_addr = object::spec_create_user_derived_object_address(
            owner_addr, derive_from
        );
        modifies object::ObjectCore[store_addr];
        modifies object::TombStone[store_addr];
        modifies object::Untransferable[store_addr];
        modifies fungible_asset::FungibleStore[store_addr];
        modifies fungible_asset::ConcurrentFungibleBalance[store_addr];
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if aborts_of<ensure_primary_store_exists<T>> (owner_addr, metadata);
        aborts_if ({
            let store =
                ..S1 |~ result_of<ensure_primary_store_exists<T>> (owner_addr, metadata);
            !aborts_of<ensure_primary_store_exists<T>> (owner_addr, metadata)
                && (S1 |~ aborts_of<may_be_unburn>(owner, store))
        });
        aborts_if ({
            let store =
                ..S1 |~ result_of<ensure_primary_store_exists<T>> (owner_addr, metadata);
            !aborts_of<ensure_primary_store_exists<T>> (owner_addr, metadata)
                && !(S1 |~ aborts_of<may_be_unburn>(owner, store))
                && (
                    S2 |~ aborts_of<dispatchable_fungible_asset::withdraw<fungible_asset::FungibleStore
                    >> (owner, store, amount)
                )
        });
        ensures ({
            let store =
                ..S1 |~ result_of<ensure_primary_store_exists<T>> (owner_addr, metadata);
            !aborts_of<ensure_primary_store_exists<T>> (owner_addr, metadata)
                && !(S1 |~ aborts_of<may_be_unburn>(owner, store))
                && !(
                    S2 |~ aborts_of<dispatchable_fungible_asset::withdraw<fungible_asset::FungibleStore
                    >> (owner, store, amount)
                ) ==>
                result
                    == (
                        S2.. |~ result_of<dispatchable_fungible_asset::withdraw<fungible_asset::FungibleStore
                        >> (owner, store, amount)
                    )
        });
        ensures ({
            let store =
                ..S1 |~ result_of<ensure_primary_store_exists<T>> (owner_addr, metadata);
            !aborts_of<ensure_primary_store_exists<T>> (owner_addr, metadata)
                && !(S1 |~ aborts_of<may_be_unburn>(owner, store))
                && !(
                    S2 |~ aborts_of<dispatchable_fungible_asset::withdraw<fungible_asset::FungibleStore
                    >> (owner, store, amount)
                ) ==>
                (
                    S2.. |~ ensures_of<dispatchable_fungible_asset::withdraw<fungible_asset::FungibleStore
                    >> (owner, store, amount, result)
                )
        });
    }

    spec withdraw_with_ref(
        transfer_ref: &0x1::fungible_asset::TransferRef, owner: address, amount: u64
    ): 0x1::fungible_asset::FungibleAsset {
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] result
            == {
                let a =
                    ..S1 |~ result_of<primary_store<fungible_asset::Metadata>> (
                        owner, fungible_asset::transfer_ref_metadata(transfer_ref)
                    );
                S1.. |~ result_of<fungible_asset::withdraw_with_ref<fungible_asset::FungibleStore
                    >> (transfer_ref, a, amount)
            };
        aborts_if [inferred] aborts_of<primary_store<fungible_asset::Metadata>> (
            owner, fungible_asset::transfer_ref_metadata(transfer_ref)
        );
    }

    spec transfer_assert_minimum_deposit<T: key>(
        sender: &signer,
        metadata: 0x1::object::Object<T>,
        recipient: address,
        amount: u64,
        expected: u64
    ) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        aborts_if [inferred = sathard] aborts_of<ensure_primary_store_exists<T>> (
            signer::address_of(sender), metadata
        );
    }

    spec create_primary_store<T: key>(
        owner_addr: address, metadata: 0x1::object::Object<T>
    ): 0x1::object::Object<0x1::fungible_asset::FungibleStore> {
        use 0x1::object;
        use 0x1::fungible_asset;
        use 0x1::features;
        pragma opaque = true, aborts_if_is_partial = false;
        let metadata_addr = object::object_address<T>(metadata);
        // The executable derives the store from the reference kept in the
        // metadata's DeriveRefPod, not directly from the Object wrapper.
        // Use that same expression to align this caller frame with
        // object::create_user_derived_object and fungible_asset::create_store.
        let derive_from = DeriveRefPod[metadata_addr].metadata_derive_ref.self;
        let store_addr = object::spec_create_user_derived_object_address(
            owner_addr, derive_from
        );
        modifies object::ObjectCore[store_addr];
        modifies fungible_asset::FungibleStore[store_addr];
        modifies fungible_asset::ConcurrentFungibleBalance[store_addr];
        modifies object::Untransferable[store_addr];
        ensures result
            == object::Object<fungible_asset::FungibleStore> { inner: store_addr };
        aborts_if [inferred] aborts_of<object::create_user_derived_object>(
            owner_addr,
            DeriveRefPod[object::object_address<T>(metadata)].metadata_derive_ref
        );
        aborts_if [inferred]!exists<DeriveRefPod>(object::object_address<T>(metadata));
        aborts_if [inferred] aborts_of<object::address_to_object<fungible_asset::Metadata
            >> (object::object_address<T>(metadata));
        aborts_if exists<fungible_asset::FungibleStore>(store_addr);
        aborts_if features::spec_is_enabled(68)
            && exists<fungible_asset::ConcurrentFungibleBalance>(store_addr);
        aborts_if exists<object::Untransferable>(store_addr);
    }

    spec create_primary_store_enabled_fungible_asset(
        constructor_ref: &0x1::object::ConstructorRef,
        maximum_supply: 0x1::option::Option<u128>,
        name: 0x1::string::String,
        symbol: 0x1::string::String,
        decimals: u8,
        icon_uri: 0x1::string::String,
        project_uri: 0x1::string::String
    ) {
        use 0x1::signer;
        use 0x1::object;
        use 0x1::fungible_asset;
        pragma opaque = true, aborts_if_is_partial = false;
        // Trusted wrapper boundary: body verification inherits the Boogie
        // String byte-vector type-generation failure from add_fungibility.
        pragma verify = false;
        let metadata_address = constructor_ref.self;
        modifies DeriveRefPod[
            signer::address_of(result_of<object::generate_signer>(constructor_ref))
        ];
        modifies fungible_asset::Metadata[metadata_address];
        modifies fungible_asset::Supply[metadata_address];
        modifies fungible_asset::ConcurrentSupply[metadata_address];
        ensures [inferred] {
            let a =
                signer::address_of(
                    S1..S2 |~ result_of<object::generate_signer>(constructor_ref)
                );
            let b = DeriveRefPod {
                metadata_derive_ref: object::generate_derive_ref(constructor_ref)
            };
            S2.. |~ publish<DeriveRefPod>(a, b)
        };
        ensures [inferred]({
            let a =
                ..S1 |~ result_of<fungible_asset::add_fungibility>(
                    constructor_ref,
                    maximum_supply,
                    name,
                    symbol,
                    decimals,
                    icon_uri,
                    project_uri
                );
            ..S1 |~ ensures_of<fungible_asset::add_fungibility>(
                constructor_ref,
                maximum_supply,
                name,
                symbol,
                decimals,
                icon_uri,
                project_uri,
                a
            )
        });
        aborts_if [inferred] S2 |~(
            exists<DeriveRefPod>(
                signer::address_of(
                    S1..S2 |~ result_of<object::generate_signer>(constructor_ref)
                )
            )
        );
        aborts_if aborts_of<fungible_asset::add_fungibility>(
            constructor_ref,
            maximum_supply,
            name,
            symbol,
            decimals,
            icon_uri,
            project_uri
        );
    }

    spec deposit_with_signer(
        owner: &signer, fa: 0x1::fungible_asset::FungibleAsset
    ) {
        use 0x1::signer;
        use 0x1::fungible_asset;
        use 0x1::dispatchable_fungible_asset;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard]({
            let a =
                ..S1 |~ result_of<ensure_primary_store_exists<fungible_asset::Metadata>> (
                    signer::address_of(owner), fungible_asset::asset_metadata(fa)
                );
            S1.. |~ ensures_of<dispatchable_fungible_asset::deposit<fungible_asset::FungibleStore
                >> (a, fa)
        });
        aborts_if [inferred = sathard] aborts_of<ensure_primary_store_exists<fungible_asset::Metadata
            >> (signer::address_of(owner), fungible_asset::asset_metadata(fa));
    }

    spec ensure_primary_store_exists<T: key>(
        owner: address, metadata: 0x1::object::Object<T>
    ): 0x1::object::Object<0x1::fungible_asset::FungibleStore> {
        use 0x1::object;
        use 0x1::fungible_asset;
        pragma opaque = true;
        let metadata_addr = object::object_address<T>(metadata);
        let derive_from = DeriveRefPod[metadata_addr].metadata_derive_ref.self;
        let store_addr = object::spec_create_user_derived_object_address(
            owner, derive_from
        );
        modifies object::ObjectCore[store_addr];
        modifies fungible_asset::FungibleStore[store_addr];
        modifies fungible_asset::ConcurrentFungibleBalance[store_addr];
        modifies object::Untransferable[store_addr];
        ensures result
            == object::Object<fungible_asset::FungibleStore> { inner: store_addr };
        ensures [inferred](
            ..S2 |~ result_of<fungible_asset::store_exists>({
                object::spec_create_user_derived_object_address(owner, metadata_addr)
            })
        ) ==>
            result
                == object::address_to_object<fungible_asset::FungibleStore>({
                    object::spec_create_user_derived_object_address(
                        owner, metadata_addr
                    )
                });
        ensures [inferred]({
            let a = S2.. |~ result_of<create_primary_store<T>> (owner, metadata);
            let b =
                ..S2 |~ result_of<fungible_asset::store_exists>({
                    object::spec_create_user_derived_object_address(owner, metadata_addr)
                });
            !b ==> result == a
        });
        aborts_if [inferred](
            ..S2 |~ result_of<fungible_asset::store_exists>({
                object::spec_create_user_derived_object_address(owner, metadata_addr)
            })
        ) && aborts_of<object::address_to_object<fungible_asset::FungibleStore>> ({
            object::spec_create_user_derived_object_address(owner, metadata_addr)
        });
    }

    spec may_be_unburn(
        owner: &signer, store: 0x1::object::Object<0x1::fungible_asset::FungibleStore>
    ) {
        use 0x1::object;
        use 0x1::fungible_asset;
        let store_addr = object::object_address<fungible_asset::FungibleStore>(store);
        modifies object::ObjectCore[store_addr];
        modifies object::TombStone[store_addr];
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if aborts_of<object::is_burnt<fungible_asset::FungibleStore>> (store);
        aborts_if !aborts_of<object::is_burnt<fungible_asset::FungibleStore>> (store)
            && (..S1 |~ result_of<object::is_burnt<fungible_asset::FungibleStore>> (store))
            && aborts_of<object::unburn<fungible_asset::FungibleStore>> (owner, store);
        ensures !aborts_of<object::is_burnt<fungible_asset::FungibleStore>> (store)
            && (..S1 |~ result_of<object::is_burnt<fungible_asset::FungibleStore>> (store))
            && !aborts_of<object::unburn<fungible_asset::FungibleStore>> (owner, store) ==>
            (
                S1.. |~ ensures_of<object::unburn<fungible_asset::FungibleStore>> (
                    owner, store
                )
            );
        ensures !aborts_of<object::is_burnt<fungible_asset::FungibleStore>> (store)
            && !(..S1 |~ result_of<object::is_burnt<fungible_asset::FungibleStore>> (store)) ==>
            exists<object::ObjectCore>(store_addr)
                == old(exists<object::ObjectCore>(store_addr));
        ensures !aborts_of<object::is_burnt<fungible_asset::FungibleStore>> (store)
            && !(..S1 |~ result_of<object::is_burnt<fungible_asset::FungibleStore>> (store))
            && old(exists<object::ObjectCore>(store_addr)) ==>
            object::ObjectCore[store_addr] == old(object::ObjectCore[store_addr]);
        ensures !aborts_of<object::is_burnt<fungible_asset::FungibleStore>> (store)
            && !(..S1 |~ result_of<object::is_burnt<fungible_asset::FungibleStore>> (store)) ==>
            !exists<object::TombStone>(store_addr);
    }
}
