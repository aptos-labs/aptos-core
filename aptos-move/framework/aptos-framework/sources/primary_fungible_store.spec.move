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
        /// The derive ref of an asset derives from the asset's own address.
        invariant forall m: address {exists<DeriveRefPod>(m)} where exists<DeriveRefPod>(m):
            global<DeriveRefPod>(m).metadata_derive_ref.self == m;
    }

    spec fun spec_primary_store_exists<T: key>(account: address, metadata: Object<T>): bool {
        fungible_asset::store_exists(spec_primary_store_address(account, metadata))
    }

    spec fun spec_primary_store_address<T: key>(owner: address, metadata: Object<T>): address {
        let metadata_addr = object::object_address(metadata);
        object::spec_create_user_derived_object_address(owner, metadata_addr)
    }

    spec primary_store_address<T: key>(owner: address, metadata: Object<T>): address {
        pragma opaque;
        aborts_if false;
        ensures result == spec_primary_store_address(owner, metadata);
    }

    spec primary_store_exists<T: key>(account: address, metadata: Object<T>): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_primary_store_exists(account, metadata);
    }

    spec primary_store<T: key>(owner: address, metadata: Object<T>): Object<FungibleStore> {
        pragma opaque;
        let store_addr = spec_primary_store_address(owner, metadata);
        aborts_if !exists<object::ObjectCore>(store_addr);
        aborts_if !object::spec_exists_at<FungibleStore>(store_addr);
        ensures object::object_address(result) == store_addr;
    }

    spec create_primary_store<T: key>(owner_addr: address, metadata: Object<T>): Object<FungibleStore> {
        let store_addr = spec_primary_store_address(owner_addr, metadata);
        include CreatePrimaryStoreAbortsIf<T>;
        ensures result.inner == store_addr;
        ensures global<object::ObjectCore>(store_addr).owner == owner_addr;
        ensures !global<object::ObjectCore>(store_addr).allow_ungated_transfer;
    }

    spec schema CreatePrimaryStoreAbortsIf<T> {
        owner_addr: address;
        metadata: Object<T>;
        let metadata_addr = metadata.inner;
        let store_addr = spec_primary_store_address(owner_addr, metadata);
        aborts_if !exists<object::ObjectCore>(metadata_addr) || !exists<Metadata>(metadata_addr);
        aborts_if !exists<DeriveRefPod>(metadata_addr);
        aborts_if exists<object::ObjectCore>(store_addr);
        aborts_if exists<object::Untransferable>(store_addr);
        aborts_if exists<FungibleStore>(store_addr);
        aborts_if std::features::spec_is_enabled(std::features::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE)
            && exists<fungible_asset::ConcurrentFungibleBalance>(store_addr);
    }

    spec withdraw<T: key>(owner: &signer, metadata: Object<T>, amount: u64): FungibleAsset {
        // Partial: the abort conditions of store creation, unburning, the ownership check and a
        // withdraw hook are not specified.
        pragma aborts_if_is_partial;
        let store_addr = spec_primary_store_address(signer::address_of(owner), metadata);
        let existing = exists<FungibleStore>(store_addr);
        let store = global<FungibleStore>(store_addr);
        aborts_if existing && store.frozen;
        aborts_if existing && fungible_asset::spec_store_no_withdraw_hook(store_addr)
            && fungible_asset::spec_balance(store_addr) < amount;
        ensures existing && fungible_asset::spec_store_no_withdraw_hook(store_addr) ==>
            fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount
            && result == FungibleAsset { metadata: store.metadata, amount };
    }

    spec deposit(owner: address, fa: FungibleAsset) {
        // Partial: the abort conditions of store creation and a deposit hook are not specified.
        pragma aborts_if_is_partial;
        let store_addr = spec_primary_store_address(owner, fa.metadata);
        let existing = exists<FungibleStore>(store_addr);
        let store = global<FungibleStore>(store_addr);
        aborts_if existing && store.frozen;
        aborts_if existing && fungible_asset::spec_store_no_deposit_hook(store_addr)
            && fa.metadata != store.metadata;
        include PrimaryDepositBalance;
    }

    /// Without a deposit hook, `fa` lands in the primary store of `owner`, which is created if missing.
    spec schema PrimaryDepositBalance {
        owner: address;
        fa: FungibleAsset;
        let store_addr = spec_primary_store_address(owner, fa.metadata);
        let existing = exists<FungibleStore>(store_addr);
        // A missing store is created with the asset's metadata.
        let no_hook = if (existing) {
            fungible_asset::spec_store_no_deposit_hook(store_addr)
        } else {
            spec_metadata_no_deposit_hook(fa.metadata.inner)
        };
        ensures no_hook ==> exists<FungibleStore>(store_addr)
            && global<FungibleStore>(store_addr).metadata == fa.metadata
            && fungible_asset::spec_balance(store_addr)
                == (if (existing) { old(fungible_asset::spec_balance(store_addr)) } else { 0 }) + fa.amount;
    }

    /// The asset at `m` has no deposit hook.
    spec fun spec_metadata_no_deposit_hook(m: address): bool {
        !exists<fungible_asset::DispatchFunctionStore>(m)
            || std::option::is_none(global<fungible_asset::DispatchFunctionStore>(m).deposit_function)
    }

    /// The store at `from` exists and the transfer runs without hooks; a missing store at `to` is created for asset `m`.
    spec fun spec_is_plain_transfer_to(from: address, to: address, m: address): bool {
        exists<FungibleStore>(from) && fungible_asset::spec_store_no_withdraw_hook(from)
            && if (exists<FungibleStore>(to)) {
                fungible_asset::spec_store_no_deposit_hook(to)
            } else {
                spec_metadata_no_deposit_hook(m)
            }
    }

    /// If `plain`, `amount` moves from the store at `from` to the store at `to`, where a store missing at `to` is
    /// created with balance `amount`; a self-transfer changes nothing.
    spec schema PlainTransferBalancesToNew {
        from: address;
        to: address;
        amount: u64;
        plain: bool;
        ensures plain && from != to ==>
            fungible_asset::spec_balance(from) == old(fungible_asset::spec_balance(from)) - amount
            && exists<FungibleStore>(to)
            && fungible_asset::spec_balance(to) == (if (old(exists<FungibleStore>(to))) {
                old(fungible_asset::spec_balance(to))
            } else { 0 }) + amount;
        ensures plain && from == to ==>
            fungible_asset::spec_balance(from) == old(fungible_asset::spec_balance(from));
    }

    /// Moves `amount` between the primary stores of `sender` and `recipient`.
    spec schema PrimaryTransfer<T> {
        sender: &signer;
        metadata: Object<T>;
        recipient: address;
        amount: u64;
        let from = spec_primary_store_address(signer::address_of(sender), metadata);
        let to = spec_primary_store_address(recipient, metadata);
        let plain = spec_is_plain_transfer_to(from, to, metadata.inner);
        aborts_if exists<FungibleStore>(from) && global<FungibleStore>(from).frozen;
        // A withdraw hook runs before the deposit check and may change the recipient's frozen flag.
        aborts_if exists<FungibleStore>(from) && exists<FungibleStore>(to)
            && fungible_asset::spec_store_no_withdraw_hook(from) && global<FungibleStore>(to).frozen;
        aborts_if plain && fungible_asset::spec_balance(from) < amount;
        include PlainTransferBalancesToNew { from, to, amount, plain };
    }

    spec transfer<T: key>(sender: &signer, metadata: Object<T>, recipient: address, amount: u64) {
        // Partial: the abort conditions of store creation, unburning, the ownership check and the
        // dispatch hooks are not specified.
        pragma aborts_if_is_partial;
        include PrimaryTransfer<T>;
    }

    spec transfer_assert_minimum_deposit<T: key>(
        sender: &signer,
        metadata: Object<T>,
        recipient: address,
        amount: u64,
        expected: u64,
    ) {
        // Partial: the abort conditions of store creation, unburning, the ownership check and the
        // dispatch hooks are not specified.
        pragma aborts_if_is_partial;
        let from = spec_primary_store_address(signer::address_of(sender), metadata);
        let to = spec_primary_store_address(recipient, metadata);
        include PrimaryTransfer<T>;
        aborts_if spec_is_plain_transfer_to(from, to, metadata.inner) && from != to && amount < expected;
    }

    /// Makes the object a fungible asset whose primary stores can be derived from it.
    spec create_primary_store_enabled_fungible_asset(
        constructor_ref: &ConstructorRef,
        maximum_supply: Option<u128>,
        name: String,
        symbol: String,
        decimals: u8,
        icon_uri: String,
        project_uri: String,
    ) {
        let addr = constructor_ref.self;
        include fungible_asset::AddFungibilityAbortsIf;
        aborts_if exists<DeriveRefPod>(addr);
        ensures exists<DeriveRefPod>(addr);
        ensures global<Metadata>(addr) == fungible_asset::Metadata { name, symbol, decimals, icon_uri, project_uri };
    }

    /// Returns the primary store of `owner`, creating it if missing.
    spec ensure_primary_store_exists<T: key>(owner: address, metadata: Object<T>): Object<FungibleStore> {
        let metadata_addr = metadata.inner;
        let store_addr = spec_primary_store_address(owner, metadata);
        let existing = exists<FungibleStore>(store_addr);
        aborts_if existing && !exists<object::ObjectCore>(store_addr);
        include !existing ==> CreatePrimaryStoreAbortsIf<T> { owner_addr: owner };
        ensures result.inner == store_addr;
        ensures exists<FungibleStore>(store_addr);
        ensures existing ==> global<FungibleStore>(store_addr) == old(global<FungibleStore>(store_addr));
        ensures !existing ==> global<FungibleStore>(store_addr).metadata.inner == metadata_addr
            && !global<FungibleStore>(store_addr).frozen
            && fungible_asset::spec_balance(store_addr) == 0
            && global<object::ObjectCore>(store_addr).owner == owner;
    }

    spec deposit_with_signer(owner: &signer, fa: FungibleAsset) {
        // Partial: the abort conditions of store creation and a deposit hook are not specified.
        pragma aborts_if_is_partial;
        let store_addr = spec_primary_store_address(signer::address_of(owner), fa.metadata);
        let existing = exists<FungibleStore>(store_addr);
        let store = global<FungibleStore>(store_addr);
        aborts_if existing && store.frozen;
        aborts_if existing && fungible_asset::spec_store_no_deposit_hook(store_addr)
            && fa.metadata != store.metadata;
        include PrimaryDepositBalance { owner: signer::address_of(owner) };
    }

    /// Mints `amount` into the primary store of `owner`, which is created if missing.
    spec mint(mint_ref: &MintRef, owner: address, amount: u64) {
        // Partial: the abort conditions of store creation and supply overflow are not specified.
        pragma aborts_if_is_partial;
        let metadata = mint_ref.metadata;
        let store_addr = spec_primary_store_address(owner, metadata);
        let existing = exists<FungibleStore>(store_addr);
        aborts_if existing && global<FungibleStore>(store_addr).frozen;
        aborts_if existing && global<FungibleStore>(store_addr).metadata != metadata;
        aborts_if amount != 0 && !exists<fungible_asset::ConcurrentSupply>(metadata.inner)
            && !exists<fungible_asset::Supply>(metadata.inner);
        ensures exists<FungibleStore>(store_addr);
        ensures fungible_asset::spec_balance(store_addr)
            == (if (existing) { old(fungible_asset::spec_balance(store_addr)) } else { 0 }) + amount;
        ensures amount != 0 ==> fungible_asset::spec_supply(metadata.inner)
            == old(fungible_asset::spec_supply(metadata.inner)) + amount;
    }

    /// Burns `amount` from the primary store of `owner`, ignoring its frozen flag.
    spec burn(burn_ref: &BurnRef, owner: address, amount: u64) {
        let metadata = burn_ref.metadata;
        let store_addr = spec_primary_store_address(owner, metadata);
        aborts_if !exists<object::ObjectCore>(store_addr);
        aborts_if !exists<FungibleStore>(store_addr);
        aborts_if fungible_asset::spec_balance(store_addr) < amount;
        aborts_if metadata != global<FungibleStore>(store_addr).metadata;
        aborts_if amount != 0 && !exists<fungible_asset::ConcurrentSupply>(metadata.inner)
            && !exists<fungible_asset::Supply>(metadata.inner);
        aborts_if amount != 0 && fungible_asset::spec_supply(metadata.inner) < amount;
        ensures fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount;
        ensures amount != 0 ==> fungible_asset::spec_supply(metadata.inner)
            == old(fungible_asset::spec_supply(metadata.inner)) - amount;
    }

    spec set_frozen_flag(transfer_ref: &TransferRef, owner: address, frozen: bool) {
        // Partial: the abort conditions of store creation are not specified.
        pragma aborts_if_is_partial;
        let store_addr = spec_primary_store_address(owner, transfer_ref.metadata);
        aborts_if exists<FungibleStore>(store_addr)
            && global<FungibleStore>(store_addr).metadata != transfer_ref.metadata;
        ensures exists<FungibleStore>(store_addr);
        ensures global<FungibleStore>(store_addr).frozen == frozen;
    }

    /// Withdraws from the primary store of `owner`, ignoring its frozen flag.
    spec withdraw_with_ref(transfer_ref: &TransferRef, owner: address, amount: u64): FungibleAsset {
        let store_addr = spec_primary_store_address(owner, transfer_ref.metadata);
        aborts_if !exists<object::ObjectCore>(store_addr);
        aborts_if !exists<FungibleStore>(store_addr);
        aborts_if transfer_ref.metadata != global<FungibleStore>(store_addr).metadata;
        aborts_if fungible_asset::spec_balance(store_addr) < amount;
        ensures fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount;
        ensures result == FungibleAsset { metadata: transfer_ref.metadata, amount };
    }

    /// Deposits into the primary store of `owner`, which is created if missing, ignoring its frozen flag.
    spec deposit_with_ref(transfer_ref: &TransferRef, owner: address, fa: FungibleAsset) {
        // Partial: the abort conditions of store creation and balance overflow are not specified.
        pragma aborts_if_is_partial;
        let store_addr = spec_primary_store_address(owner, transfer_ref.metadata);
        let existing = exists<FungibleStore>(store_addr);
        aborts_if transfer_ref.metadata != fa.metadata;
        aborts_if existing && global<FungibleStore>(store_addr).metadata != fa.metadata;
        ensures exists<FungibleStore>(store_addr);
        ensures fungible_asset::spec_balance(store_addr)
            == (if (existing) { old(fungible_asset::spec_balance(store_addr)) } else { 0 }) + fa.amount;
    }

    /// Moves `amount` between the primary stores of `from` and `to`, ignoring their frozen flags.
    spec transfer_with_ref(transfer_ref: &TransferRef, from: address, to: address, amount: u64) {
        // Partial: the abort conditions of store creation and balance overflow are not specified.
        pragma aborts_if_is_partial;
        let from_addr = spec_primary_store_address(from, transfer_ref.metadata);
        let to_addr = spec_primary_store_address(to, transfer_ref.metadata);
        aborts_if !exists<FungibleStore>(from_addr);
        aborts_if transfer_ref.metadata != global<FungibleStore>(from_addr).metadata;
        aborts_if fungible_asset::spec_balance(from_addr) < amount;
        ensures from_addr != to_addr ==>
            fungible_asset::spec_balance(from_addr) == old(fungible_asset::spec_balance(from_addr)) - amount
            && fungible_asset::spec_balance(to_addr) == (if (old(exists<FungibleStore>(to_addr))) {
                old(fungible_asset::spec_balance(to_addr))
            } else { 0 }) + amount;
        ensures from_addr == to_addr ==>
            fungible_asset::spec_balance(from_addr) == old(fungible_asset::spec_balance(from_addr));
    }

    spec upgrade_to_concurrent(owner: &signer, metadata: Object<Metadata>) {
        // Partial: the abort conditions of store creation and the upgrade are not specified.
        pragma aborts_if_is_partial;
        let store_addr = spec_primary_store_address(signer::address_of(owner), metadata);
        ensures exists<fungible_asset::ConcurrentFungibleBalance>(store_addr);
        ensures old(exists<FungibleStore>(store_addr)) ==> fungible_asset::spec_balance(store_addr)
            == old(fungible_asset::spec_balance(store_addr));
    }
}
