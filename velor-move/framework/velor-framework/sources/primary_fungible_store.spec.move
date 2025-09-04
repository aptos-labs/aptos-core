spec velor_framework::primary_fungible_store {
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
        // TODO: verification disabled until this module is specified.
        pragma verify = false;
    }

    spec fun spec_primary_store_exists<T: key>(account: address, metadata: Object<T>): bool {
        fungible_asset::store_exists(spec_primary_store_address(account, metadata))
    }

    spec fun spec_primary_store_address<T: key>(owner: address, metadata: Object<T>): address {
        let metadata_addr = object::object_address(metadata);
        object::spec_create_user_derived_object_address(owner, metadata_addr)
    }
}
