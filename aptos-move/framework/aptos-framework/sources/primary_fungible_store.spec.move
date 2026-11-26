spec aptos_framework::primary_fungible_store {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Creating a fungible asset with primary store support should initiate a derived reference and store it
    /// under the metadata object.
    /// Criticality: Medium
    /// Implementation: The function create_primary_store_enabled_fungible_asset makes an existing object, fungible, via
    /// the fungible_asset::add_fungibility function and initializes the DeriveRefPod resource by generating a DeriveRef
    /// for the object and then stores it under the object address.
    /// Enforcement: Formally verified via [high-level-req-1.1](create_primary_store_enabled_fungible_asset) that the
    /// DeriveRefPod is stored under the metadata object, and via the [high-level-req-1.2](module) invariant that its
    /// derive ref derives from that object.
    ///
    /// No.: 2
    /// Requirement: Fetching and creating a primary fungible store of an asset should only succeed if the object supports
    /// primary store.
    /// Criticality: Low
    /// Implementation: The function create_primary_store is used to create a primary store by borrowing the DeriveRef
    /// resource from the object. In case the resource does not exist, creation will fail. The function
    /// ensure_primary_store_exists is used to fetch the primary store if it exists, otherwise it will create one via
    /// the create_primary function.
    /// Enforcement: Formally verified via [high-level-req-2.1](CreatePrimaryStoreAbortsIf) that it aborts if the
    /// DeriveRefPod doesn't exist, and via [high-level-req-2.2](CreatePrimaryStoreAbortsIf) that it aborts if the
    /// FungibleStore resource exists already under the object address.
    ///
    /// No.: 3
    /// Requirement: It should be possible to create a primary store to hold a fungible asset.
    /// Criticality: Medium
    /// Implementation: The function create_primary_store borrows the DeriveRef resource from DeriveRefPod and then
    /// creates the store which is returned.
    /// Enforcement: Formally verified via [high-level-req-3.1](create_primary_store) and
    /// [high-level-req-3.2](create_primary_store) that it returns the newly created FungibleStore.
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
    /// Enforcement: Formally verified via [high-level-req-5.1](burn) and [high-level-req-5.2](withdraw_with_ref) that
    /// burning and withdrawing with a ref abort if the primary store doesn't exist, and via
    /// [high-level-req-2.1](CreatePrimaryStoreAbortsIf) that mint, deposit_with_ref, transfer_with_ref and
    /// set_frozen_flag create a missing store only for an asset with primary store support. Audited for withdraw,
    /// deposit and transfer, whose store-creation aborts are not specified.
    ///
    /// No.: 6
    /// Requirement: The action of depositing a fungible asset of the same type as the store should never fail if the store
    /// is not frozen.
    /// Criticality: Medium
    /// Implementation: The function deposit fetches the owner's store, if it doesn't exist it will be created, and then
    /// deposits the fungible asset to it. The function deposit_with_ref fetches the owner's store, if it doesn't exist
    /// it will be created, and then deposit the fungible asset via the fungible_asset::deposit_with_ref function.
    /// Depositing fails if the metadata of the FungibleStore and FungibleAsset differs.
    /// Enforcement: Formally verified that deposit aborts if the store is frozen via [high-level-req-6.1](deposit)
    /// and, without a deposit hook, if the metadata of the store and the asset differs via
    /// [high-level-req-6.2](deposit); that deposit_with_ref aborts on differing metadata via
    /// [high-level-req-6.3](deposit_with_ref); and that the store balance is increased by the deposit amount via
    /// [high-level-req-6.4](PrimaryDepositBalance) (deposit without a deposit hook) and
    /// [high-level-req-6.5](deposit_with_ref). That deposit fails for no other reason is audited, as its abort
    /// conditions are partial.
    ///
    /// No.: 7
    /// Requirement: Withdrawing should only be allowed to the owner of an existing store with sufficient balance.
    /// Criticality: Critical
    /// Implementation: The withdraw function fetches the owner's store via the primary_store function and then calls
    /// fungible_asset::withdraw which validates the owner of the store, checks the frozen status and the balance of the
    /// store. The withdraw_with_ref function fetches the store of the owner via primary_store function and calls the
    /// fungible_asset::withdraw_with_ref which validates transfer_ref's metadata with the withdrawing stores metadata,
    /// and the balance of the store.
    /// Enforcement: Formally verified that withdraw aborts if the store is frozen via [high-level-req-7.1](withdraw);
    /// that withdraw_with_ref aborts if the transfer ref's metadata doesn't match the store's metadata via
    /// [high-level-req-7.2](withdraw_with_ref); that it aborts if the store doesn't have sufficient balance via
    /// [high-level-req-7.3](withdraw) (without a withdraw hook) and [high-level-req-7.4](withdraw_with_ref); and that
    /// the store balance is decreased by the amount withdrawn via [high-level-req-7.5](withdraw) (without a withdraw
    /// hook) and [high-level-req-7.6](withdraw_with_ref). Audited that it aborts if the owner doesn't own the store
    /// and that the store is unburned (withdraw).
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
    /// Enforcement: Formally verified that the sender's store balance is decreased by the specified amount and the
    /// recipient's increased by it via [high-level-req-9.1](PlainTransferBalancesToNew) (transfer without dispatch
    /// hooks) and [high-level-req-9.2](transfer_with_ref). Audited the owner check and that the sender's store is
    /// unburned (transfer), and transfers with dispatch hooks.
    ///
    /// No.: 10
    /// Requirement: Minting an amount of assets to an unfrozen store is only allowed with a valid mint reference.
    /// Criticality: High
    /// Implementation: The mint function fetches the primary store and calls the fungible_asset::mint_to, which mints
    /// with MintRef's metadata which internally validates the amount and the increases the total supply of the asset.
    /// And the minted asset is deposited to the provided store by validating that the store is unfrozen and the store's
    /// metadata is the same as the depositing asset's metadata.
    /// Enforcement: Formally verified via [high-level-req-10.1](mint) that it aborts if the store is frozen, via
    /// [high-level-req-10.2](mint) that it aborts if the mint_ref's metadata is not the same as the store's metadata,
    /// via [high-level-req-10.3](mint) that the balance of the store is increased by the minted amount, and via
    /// [high-level-req-10.4](mint) that the asset's total supply is increased by the amount minted.
    ///
    /// No.: 11
    /// Requirement: Burning an amount of assets from an existing unfrozen store is only allowed with a valid burn
    /// reference.
    /// Criticality: High
    /// Implementation: The burn function fetches the primary store and calls the fungible_asset::burn_from function
    /// which withdraws the amount from the store while enforcing that the store has enough balance and burns the
    /// withdrawn asset after validating the asset's metadata and the BurnRef's metadata followed by decreasing the
    /// supply of the asset.
    /// Enforcement: Formally verified via [high-level-req-11.1](burn) that it aborts if the store doesn't have enough
    /// balance, via [high-level-req-11.2](burn) that it aborts if the metadata of the store is not same as the
    /// BurnRef's metadata, via [high-level-req-11.3](burn) that the store's balance is reduced by the amount burned,
    /// and via [high-level-req-11.4](burn) that the total supply of the asset is decreased by it.
    ///
    /// No.: 12
    /// Requirement: Setting the frozen flag of a store is only allowed with a valid reference.
    /// Criticality: High
    /// Implementation: The function set_frozen_flag fetches the primary store and calls fungible_asset::set_frozen_flag
    /// which validates the TransferRef's metadata with the store's metadata and then updates the frozen flag.
    /// Enforcement: Formally verified via [high-level-req-12.1](set_frozen_flag) and
    /// [high-level-req-12.2](set_frozen_flag).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        /// The derive ref of an asset derives from the asset's own address.
        /// [high-level-req-1.2]
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
        /// [high-level-req-3.1]
        ensures result.inner == store_addr;
        /// [high-level-req-3.2]
        ensures exists<FungibleStore>(store_addr);
        ensures global<object::ObjectCore>(store_addr).owner == owner_addr;
        ensures !global<object::ObjectCore>(store_addr).allow_ungated_transfer;
    }

    spec schema CreatePrimaryStoreAbortsIf<T> {
        owner_addr: address;
        metadata: Object<T>;
        let metadata_addr = metadata.inner;
        let store_addr = spec_primary_store_address(owner_addr, metadata);
        aborts_if !exists<object::ObjectCore>(metadata_addr) || !exists<Metadata>(metadata_addr);
        /// [high-level-req-2.1]
        aborts_if !exists<DeriveRefPod>(metadata_addr);
        aborts_if exists<object::ObjectCore>(store_addr);
        aborts_if exists<object::Untransferable>(store_addr);
        /// [high-level-req-2.2]
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
        /// [high-level-req-7.1]
        aborts_if existing && store.frozen;
        /// [high-level-req-7.3]
        aborts_if existing && fungible_asset::spec_store_no_withdraw_hook(store_addr)
            && fungible_asset::spec_balance(store_addr) < amount;
        /// [high-level-req-7.5]
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
        /// [high-level-req-6.1]
        aborts_if existing && store.frozen;
        /// [high-level-req-6.2]
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
        /// [high-level-req-6.4]
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
        /// [high-level-req-9.1]
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
        modifies global<Metadata>(addr);
        modifies global<fungible_asset::Supply>(addr);
        modifies global<fungible_asset::ConcurrentSupply>(addr);
        modifies global<DeriveRefPod>(addr);
        include fungible_asset::AddFungibilityAbortsIf;
        aborts_if exists<DeriveRefPod>(addr);
        /// [high-level-req-1.1]
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
        let metadata = mint_ref.metadata;
        let store_addr = spec_primary_store_address(owner, metadata);
        let existing = exists<FungibleStore>(store_addr);
        aborts_if existing && !exists<object::ObjectCore>(store_addr);
        include !existing ==> CreatePrimaryStoreAbortsIf<Metadata> { owner_addr: owner };
        /// [high-level-req-10.1]
        aborts_if existing && global<FungibleStore>(store_addr).frozen;
        /// [high-level-req-10.2]
        aborts_if existing && global<FungibleStore>(store_addr).metadata != metadata;
        include fungible_asset::IncreaseSupplyAbortsIf { metadata_addr: metadata.inner };
        // A new store starts empty, so only an existing one can overflow.
        include existing ==> fungible_asset::UncheckedDepositAbortsIf {
            store_addr, fa: FungibleAsset { metadata, amount }
        };
        ensures exists<FungibleStore>(store_addr);
        /// [high-level-req-10.3]
        ensures fungible_asset::spec_balance(store_addr)
            == (if (existing) { old(fungible_asset::spec_balance(store_addr)) } else { 0 }) + amount;
        /// [high-level-req-10.4]
        ensures amount != 0 ==> fungible_asset::spec_supply(metadata.inner)
            == old(fungible_asset::spec_supply(metadata.inner)) + amount;
    }

    /// Burns `amount` from the primary store of `owner`, ignoring its frozen flag.
    spec burn(burn_ref: &BurnRef, owner: address, amount: u64) {
        let metadata = burn_ref.metadata;
        let store_addr = spec_primary_store_address(owner, metadata);
        aborts_if !exists<object::ObjectCore>(store_addr);
        /// [high-level-req-5.1]
        aborts_if !exists<FungibleStore>(store_addr);
        /// [high-level-req-11.1]
        aborts_if fungible_asset::spec_balance(store_addr) < amount;
        /// [high-level-req-11.2]
        aborts_if metadata != global<FungibleStore>(store_addr).metadata;
        aborts_if amount != 0 && !exists<fungible_asset::ConcurrentSupply>(metadata.inner)
            && !exists<fungible_asset::Supply>(metadata.inner);
        aborts_if amount != 0 && fungible_asset::spec_supply(metadata.inner) < amount;
        /// [high-level-req-11.3]
        ensures fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount;
        /// [high-level-req-11.4]
        ensures amount != 0 ==> fungible_asset::spec_supply(metadata.inner)
            == old(fungible_asset::spec_supply(metadata.inner)) - amount;
    }

    spec set_frozen_flag(transfer_ref: &TransferRef, owner: address, frozen: bool) {
        let metadata = transfer_ref.metadata;
        let store_addr = spec_primary_store_address(owner, metadata);
        let existing = exists<FungibleStore>(store_addr);
        aborts_if existing && !exists<object::ObjectCore>(store_addr);
        include !existing ==> CreatePrimaryStoreAbortsIf<Metadata> { owner_addr: owner };
        /// [high-level-req-12.1]
        aborts_if exists<FungibleStore>(store_addr)
            && global<FungibleStore>(store_addr).metadata != transfer_ref.metadata;
        ensures exists<FungibleStore>(store_addr);
        /// [high-level-req-12.2]
        ensures global<FungibleStore>(store_addr).frozen == frozen;
    }

    /// Withdraws from the primary store of `owner`, ignoring its frozen flag.
    spec withdraw_with_ref(transfer_ref: &TransferRef, owner: address, amount: u64): FungibleAsset {
        let store_addr = spec_primary_store_address(owner, transfer_ref.metadata);
        aborts_if !exists<object::ObjectCore>(store_addr);
        /// [high-level-req-5.2]
        aborts_if !exists<FungibleStore>(store_addr);
        /// [high-level-req-7.2]
        aborts_if transfer_ref.metadata != global<FungibleStore>(store_addr).metadata;
        /// [high-level-req-7.4]
        aborts_if fungible_asset::spec_balance(store_addr) < amount;
        /// [high-level-req-7.6]
        ensures fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount;
        ensures result == FungibleAsset { metadata: transfer_ref.metadata, amount };
    }

    /// Deposits into the primary store of `owner`, which is created if missing, ignoring its frozen flag.
    spec deposit_with_ref(transfer_ref: &TransferRef, owner: address, fa: FungibleAsset) {
        let metadata = transfer_ref.metadata;
        let store_addr = spec_primary_store_address(owner, metadata);
        let existing = exists<FungibleStore>(store_addr);
        aborts_if existing && !exists<object::ObjectCore>(store_addr);
        include !existing ==> CreatePrimaryStoreAbortsIf<Metadata> { owner_addr: owner };
        aborts_if transfer_ref.metadata != fa.metadata;
        /// [high-level-req-6.3]
        aborts_if existing && global<FungibleStore>(store_addr).metadata != fa.metadata;
        // A new store starts empty, so only an existing one can overflow.
        include existing ==> fungible_asset::UncheckedDepositAbortsIf { store_addr };
        ensures exists<FungibleStore>(store_addr);
        /// [high-level-req-6.5]
        ensures fungible_asset::spec_balance(store_addr)
            == (if (existing) { old(fungible_asset::spec_balance(store_addr)) } else { 0 }) + fa.amount;
    }

    /// Moves `amount` between the primary stores of `from` and `to`, ignoring their frozen flags.
    spec transfer_with_ref(transfer_ref: &TransferRef, from: address, to: address, amount: u64) {
        let metadata = transfer_ref.metadata;
        let from_addr = spec_primary_store_address(from, metadata);
        let to_addr = spec_primary_store_address(to, metadata);
        let to_existing = exists<FungibleStore>(to_addr);
        let to_concurrent = global<fungible_asset::ConcurrentFungibleBalance>(to_addr).balance;
        aborts_if !exists<object::ObjectCore>(from_addr);
        aborts_if !exists<FungibleStore>(from_addr);
        aborts_if to_existing && !exists<object::ObjectCore>(to_addr);
        include !to_existing ==> CreatePrimaryStoreAbortsIf<Metadata> { owner_addr: to };
        aborts_if transfer_ref.metadata != global<FungibleStore>(from_addr).metadata;
        aborts_if fungible_asset::spec_balance(from_addr) < amount;
        // A new recipient store starts empty, so only an existing one can overflow.
        include to_existing && from_addr != to_addr ==> fungible_asset::UncheckedDepositAbortsIf {
            store_addr: to_addr, fa: FungibleAsset { metadata, amount }
        };
        // For a self-transfer the deposit runs on the reduced balance.
        aborts_if amount != 0 && from_addr == to_addr && fungible_asset::spec_is_concurrent_balance(to_addr)
            && aptos_framework::aggregator_v2::spec_get_value(to_concurrent)
                > aptos_framework::aggregator_v2::spec_get_max_value(to_concurrent);
        /// [high-level-req-9.2]
        ensures from_addr != to_addr ==>
            fungible_asset::spec_balance(from_addr) == old(fungible_asset::spec_balance(from_addr)) - amount
            && fungible_asset::spec_balance(to_addr) == (if (old(exists<FungibleStore>(to_addr))) {
                old(fungible_asset::spec_balance(to_addr))
            } else { 0 }) + amount;
        ensures from_addr == to_addr ==>
            fungible_asset::spec_balance(from_addr) == old(fungible_asset::spec_balance(from_addr));
    }

    spec upgrade_to_concurrent(owner: &signer, metadata: Object<Metadata>) {
        let owner_address = signer::address_of(owner);
        let store_addr = spec_primary_store_address(owner_address, metadata);
        let existing = exists<FungibleStore>(store_addr);
        let concurrent = exists<fungible_asset::ConcurrentFungibleBalance>(store_addr);
        let created_concurrent = !existing
            && std::features::spec_is_enabled(std::features::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE);
        aborts_if existing && !exists<object::ObjectCore>(store_addr);
        aborts_if existing && !concurrent && object::spec_owns_aborts(store_addr, owner_address);
        aborts_if existing && !concurrent && !object::spec_owns(store_addr, owner_address);
        aborts_if existing && !concurrent && global<FungibleStore>(store_addr).frozen;
        aborts_if !concurrent && !created_concurrent
            && !std::features::spec_is_enabled(std::features::CONCURRENT_FUNGIBLE_BALANCE);
        include !existing ==> CreatePrimaryStoreAbortsIf<Metadata> { owner_addr: signer::address_of(owner) };
        ensures exists<fungible_asset::ConcurrentFungibleBalance>(store_addr);
        ensures old(exists<FungibleStore>(store_addr)) ==> fungible_asset::spec_balance(store_addr)
            == old(fungible_asset::spec_balance(store_addr));
    }
}
