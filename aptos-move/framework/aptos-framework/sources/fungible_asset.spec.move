spec aptos_framework::fungible_asset {
    /// <high-level-req>
    /// No.: 1
    /// Property: The metadata associated with the fungible asset is subject to precise size constraints.
    /// Criticality: Medium
    /// Implementation: The add_fungibility function has size limitations for the name, symbol, number of decimals,
    /// icon_uri, and project_uri field of the Metadata resource.
    /// Enforcement: Formally verified via [high-level-req-1](add_fungibility).
    ///
    /// No.: 2
    /// Property: Adding fungibility to an existing object should initialize the metadata and supply resources and store
    /// them under the metadata object address.
    /// Criticality: Low
    /// Implementation: The add_fungibility function initializes the Metadata and Supply resources and moves them under
    /// the metadata object.
    /// Enforcement: Formally verified via [high-level-req-2](add_fungibility).
    ///
    /// No.: 3
    /// Property: Generating mint, burn and transfer references can only be done at object creation time and if the
    /// object was added fungibility.
    /// Criticality: Low
    /// Implementation: The following functions generate the related references of the Metadata object: 1.
    /// generate_mint_ref 2. generate_burn_ref 3. generate_transfer_ref
    /// Enforcement: Formally verified via [high-level-req-3.1](generate_mint_ref),
    /// [high-level-req-3.2](generate_burn_ref), and [high-level-req-3.3](generate_transfer_ref)
    ///
    /// No.: 4
    /// Property: Only the owner of a store should be allowed to withdraw fungible assets from it.
    /// Criticality: High
    /// Implementation: The withdraw function ensures that the signer owns the store by asserting that the object
    /// address matches the address of the signer.
    /// Enforcement: Formally verified via [high-level-req-4](withdraw).
    ///
    /// No.: 5
    /// Property: The transfer, withdrawal and deposit operation should never change the current supply of the fungible
    /// asset.
    /// Criticality: High
    /// Implementation: The transfer function withdraws the fungible assets from the store and deposits them to the
    /// receiver. The withdraw function extracts the fungible asset from the fungible asset store. The deposit function
    /// adds the balance to the fungible asset store.
    /// Enforcement: Formally verified via [high-level-req-5.1](withdraw), [high-level-req-5.1](deposit), and
    /// [high-level-req-5.3](transfer).
    ///
    /// No.: 6
    /// Property: The owner of the store should only be able to withdraw a certain amount if its store has sufficient
    /// balance and is not frozen, unless the withdrawal is performed with a reference, and afterwards the store balance
    /// should be decreased.
    /// Criticality: High
    /// Implementation: The withdraw function ensures that the store is not frozen before calling withdraw_internal
    /// which ensures that the withdrawing amount is greater than 0 and less than the total balance from the store.
    /// The withdraw_with_ref ensures that the reference's metadata matches the store metadata.
    /// Enforcement: Audited that it aborts if the withdrawing store is frozen. Audited that it aborts if the store
    /// doesn't have sufficient balance. Audited that the balance of the withdrawing store is reduced by amount.
    ///
    /// No.: 7
    /// Property: Only the same type of fungible assets should be deposited in a fungible asset store, if the store is
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
    /// Property: An object should only be allowed to hold one store for fungible assets.
    /// Criticality: Medium
    /// Implementation: The create_store function initializes a new FungibleStore resource and moves it under the
    /// object address.
    /// Enforcement: Audited that the resource was moved under the object.
    ///
    /// No.: 9
    /// Property: When a new store is created, the balance should be set by default to the value zero.
    /// Criticality: High
    /// Implementation: The create_store function initializes a new fungible asset store with zero balance and stores it
    /// under the given construtorRef object.
    /// Enforcement: Audited that the store is properly initialized with zero balance.
    ///
    /// No.: 10
    /// Property: A store should only be deleted if it's balance is zero.
    /// Criticality: Medium
    /// Implementation: The remove_store function validates the store's balance and removes the store under the object
    /// address.
    /// Enforcement: Audited that aborts if the balance of the store is not zero. Audited that store is removed from the
    /// object address.
    ///
    /// No.: 11
    /// Property: Minting and burning should alter the total supply value, and the store balances.
    /// Criticality: High
    /// Implementation: The mint process increases the total supply by the amount minted using the increase_supply
    /// function. The burn process withdraws the burn amount from the given store and decreases the total supply by the
    /// amount burned using the decrease_supply function.
    /// Enforcement: Audited the mint and burn functions that the supply was adjusted accordingly.
    ///
    /// No.: 12
    /// Property: It must not be possible to burn an amount of fungible assets larger than their current supply.
    /// Criticality: High
    /// Implementation: The burn process ensures that the store has enough balance to burn, by asserting that the
    /// supply.current >= amount inside the decrease_supply function.
    /// Enforcement: Audited that it aborts if the provided store doesn't have sufficient balance.
    ///
    /// No.: 13
    /// Property: Enabling or disabling store's frozen status should only be done with a valid transfer reference.
    /// Criticality: High
    /// Implementation: The set_frozen_flag function ensures that the TransferRef is provided via function argument and
    /// that the store's metadata matches the metadata from the reference. It then proceeds to update the frozen flag of
    /// the store.
    /// Enforcement: Audited that it aborts if the metadata doesn't match. Audited that the frozen flag is updated properly.
    ///
    /// No.: 14
    /// Property: Extracting a specific amount from the fungible asset should be possible only if the total amount that
    /// it holds is greater or equal to the provided amount.
    /// Criticality: High
    /// Implementation: The extract function validates that the fungible asset has enough balance to extract and then
    /// updates it by subtracting the extracted amount.
    /// Enforcement: Audited that it aborts if the asset didn't have sufficient balance. Audited that the balance of the asset is updated. Audited that the extract function returns the extracted asset.
    ///
    /// No.: 15
    /// Property: Merging two fungible assets should only be possible if both share the same metadata.
    /// Criticality: Medium
    /// Implementation: The merge function validates the metadata of the src and dst asset.
    /// Enforcement: Audited that it aborts if the metadata of the src and dst are not the same.
    ///
    /// No.: 16
    /// Property: Post merging two fungible assets, the source asset should have the amount value equal to the sum of
    /// the two.
    /// Criticality: High
    /// Implementation: The merge function increases dst_fungible_asset.amount by src_fungible_asset.amount.
    /// Enforcement: Audited that the dst_fungible_asset balance is increased by amount.
    ///
    /// No.: 17
    /// Property: Fungible assets with zero balance should be destroyed when the amount reaches value 0.
    /// Criticality: Medium
    /// Implementation: The destroy_zero ensures that the balance of the asset has the value 0 and destroy the asset.
    /// Enforcement: Audited that it aborts if the balance of the asset is non zero.
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec fun spec_exists_at<T: key>(object: address): bool;

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
        aborts_if (object::can_generate_delete_ref(constructor_ref));
        aborts_if (string::length(name) > MAX_NAME_LENGTH);
        aborts_if (string::length(symbol) > MAX_SYMBOL_LENGTH);
        aborts_if (decimals > MAX_DECIMALS);
        aborts_if (string::length(icon_uri) > MAX_URI_LENGTH);
        aborts_if (string::length(project_uri) > MAX_URI_LENGTH);
        let contructor_ref_addr = object::address_from_constructor_ref(constructor_ref);
        aborts_if !exists<object::ObjectCore>(contructor_ref_addr);
        aborts_if !object::spec_exists_at<Metadata>(contructor_ref_addr);
        aborts_if exists<Metadata>(contructor_ref_addr);
        // aborts_if (features::spec_is_enabled(features::CONCURRENT_ASSETS)) ==> exists<ConcurrentSupply>(contructor_ref_addr);
        // aborts_if !(features::spec_is_enabled(features::CONCURRENT_ASSETS)) ==> exists<Supply>(contructor_ref_addr);

        /// [high-level-req-2]
        ensures exists<Metadata>(contructor_ref_addr);
        ensures (features::spec_is_enabled(features::CONCURRENT_ASSETS)) ==> exists<ConcurrentSupply>(contructor_ref_addr);
        ensures !(features::spec_is_enabled(features::CONCURRENT_ASSETS)) ==> exists<Supply>(contructor_ref_addr);
    }

    spec generate_mint_ref(constructor_ref: &ConstructorRef): MintRef {
        let contructor_ref_addr = object::address_from_constructor_ref(constructor_ref);
        aborts_if !exists<object::ObjectCore>(contructor_ref_addr);
        aborts_if !object::spec_exists_at<Metadata>(contructor_ref_addr);
    }

    spec generate_burn_ref(constructor_ref: &ConstructorRef): BurnRef {
        let contructor_ref_addr = object::address_from_constructor_ref(constructor_ref);
        aborts_if !exists<object::ObjectCore>(contructor_ref_addr);
        aborts_if !object::spec_exists_at<Metadata>(contructor_ref_addr);
    }

    spec generate_transfer_ref(constructor_ref: &ConstructorRef): TransferRef {
        let contructor_ref_addr = object::address_from_constructor_ref(constructor_ref);
        aborts_if !exists<object::ObjectCore>(contructor_ref_addr);
        aborts_if !object::spec_exists_at<Metadata>(contructor_ref_addr);
    }

    spec supply<T: key>(metadata: Object<T>): Option<u128> {
        pragma aborts_if_is_partial;
    }

    spec maximum<T: key>(metadata: Object<T>): Option<u128> {
        pragma aborts_if_is_partial;
    }

    spec name<T: key>(metadata: Object<T>): String {
        pragma aborts_if_is_partial;
    }


    spec symbol<T: key>(metadata: Object<T>): String {
        pragma aborts_if_is_partial;
    }

    spec decimals<T: key>(metadata: Object<T>): u8 {
        pragma aborts_if_is_partial;
    }

    spec store_exists(store: address): bool {
        pragma aborts_if_is_partial;
    }

    spec metadata_from_asset(fa: &FungibleAsset): Object<Metadata> {
        pragma aborts_if_is_partial;
    }

    spec store_metadata<T: key>(store: Object<T>): Object<Metadata> {
        pragma aborts_if_is_partial;
    }

    spec amount(fa: &FungibleAsset): u64 {
        pragma aborts_if_is_partial;
    }

    spec balance<T: key>(store: Object<T>): u64 {
        pragma aborts_if_is_partial;
    }

    spec is_frozen<T: key>(store: Object<T>): bool {
        pragma aborts_if_is_partial;
    }

    spec asset_metadata(fa: &FungibleAsset): Object<Metadata> {
        pragma aborts_if_is_partial;
    }

    spec mint_ref_metadata(ref: &MintRef): Object<Metadata> {
        pragma aborts_if_is_partial;
    }

    spec transfer_ref_metadata(ref: &TransferRef): Object<Metadata> {
        pragma aborts_if_is_partial;
    }

    spec burn_ref_metadata(ref: &BurnRef): Object<Metadata> {
        pragma aborts_if_is_partial;
    }

    spec transfer<T: key>(
        sender: &signer,
        from: Object<T>,
        to: Object<T>,
        amount: u64,
        ) {
        pragma aborts_if_is_partial;

        let from_addr = from.inner;
        let supply = global<ConcurrentSupply>(from_addr);
        let post post_supply = global<ConcurrentSupply>(from_addr);
        /// [high-level-req-5.3]
        ensures post_supply == supply;
    }

    spec create_store<T: key>(
        constructor_ref: &ConstructorRef,
        metadata: Object<T>,
        ): Object<FungibleStore> {
        pragma aborts_if_is_partial;
    }

    spec remove_store(delete_ref: &DeleteRef) {
        pragma aborts_if_is_partial;
    }

    spec withdraw<T: key>(
        owner: &signer,
        store: Object<T>,
        amount: u64,
        ): FungibleAsset {
        pragma aborts_if_is_partial;

        let current_address_0 = store.inner;
        let object_0 = global<object::ObjectCore>(current_address_0);
        let current_address = object_0.owner;
        /// [high-level-req-4]
        aborts_if store.inner != signer::address_of(owner) && !exists<object::ObjectCore>(store.inner);
        aborts_if !exists<FungibleStore>(current_address_0);
        aborts_if !exists<FungibleAssetEvents>(current_address_0);

        aborts_if (amount == 0);
        aborts_if (is_frozen(store));

        let fungible_store = global<FungibleStore>(current_address_0);
        aborts_if (fungible_store.balance < amount);

        let supply = global<ConcurrentSupply>(current_address_0);
        let post post_supply = global<ConcurrentSupply>(current_address_0);
        /// [high-level-req-5.1]
        ensures post_supply == supply;
    }

    spec deposit<T: key>(store: Object<T>, fa: FungibleAsset) {
        pragma aborts_if_is_partial;

        let reciepient_addr = store.inner;
        let supply = global<ConcurrentSupply>(reciepient_addr);
        let post post_supply = global<ConcurrentSupply>(reciepient_addr);
        /// [high-level-req-5.2]
        ensures post_supply == supply;
    }

    spec mint(ref: &MintRef, amount: u64): FungibleAsset {
        pragma aborts_if_is_partial;
    }

    spec mint_to<T: key>(ref: &MintRef, store: Object<T>, amount: u64) {
        pragma aborts_if_is_partial;
    }

    spec set_frozen_flag<T: key>(
        ref: &TransferRef,
        store: Object<T>,
        frozen: bool,
        ) {
        pragma aborts_if_is_partial;
    }

    spec burn(ref: &BurnRef, fa: FungibleAsset) {
        pragma aborts_if_is_partial;
    }

    spec burn_from<T: key>(
        ref: &BurnRef,
        store: Object<T>,
        amount: u64
        ) {
        pragma aborts_if_is_partial;
    }

    spec withdraw_with_ref<T: key>(
        ref: &TransferRef,
        store: Object<T>,
        amount: u64
        ): FungibleAsset  {
        pragma aborts_if_is_partial;
    }

    spec deposit_with_ref<T: key>(
        ref: &TransferRef,
        store: Object<T>,
        fa: FungibleAsset
        ) {
        pragma aborts_if_is_partial;
    }

    spec transfer_with_ref<T: key>(
        transfer_ref: &TransferRef,
        from: Object<T>,
        to: Object<T>,
        amount: u64,
        ) {
        pragma aborts_if_is_partial;
    }

    spec zero<T: key>(metadata: Object<T>): FungibleAsset {
        pragma aborts_if_is_partial;
    }

    spec extract(fungible_asset: &mut FungibleAsset, amount: u64): FungibleAsset {
        pragma aborts_if_is_partial;
    }

    spec merge(dst_fungible_asset: &mut FungibleAsset, src_fungible_asset: FungibleAsset) {
        pragma aborts_if_is_partial;
    }

    spec destroy_zero(fungible_asset: FungibleAsset) {
        pragma aborts_if_is_partial;
    }

    spec deposit_internal<T: key>(store: Object<T>, fa: FungibleAsset) {
        pragma aborts_if_is_partial;

        let store_addr = object::object_address(store);
        let store = global<FungibleStore>(store_addr);
        let amount = fa.amount;
        let post store_balance = global<FungibleStore>(store_addr).balance;
        ensures store_balance == store.balance + amount;
    }

    spec withdraw_internal(
        store_addr: address,
        amount: u64,
        ): FungibleAsset {
        pragma aborts_if_is_partial;
        aborts_if (amount == 0);

        let store = global<FungibleStore>(store_addr);
        aborts_if (store.balance < amount);
        aborts_if !exists<FungibleStore>(store_addr);
        aborts_if !exists<FungibleAssetEvents>(store_addr);

        let post store_balance = global<FungibleStore>(store_addr).balance;
        ensures store_balance == store.balance - amount;
    }

    spec increase_supply<T: key>(metadata: &Object<T>, amount: u64) {
        pragma aborts_if_is_partial;
    }

    spec decrease_supply<T: key>(metadata: &Object<T>, amount: u64) {
        pragma aborts_if_is_partial;
    }

    spec upgrade_to_concurrent(
        ref: &ExtendRef,
        ) {
        pragma aborts_if_is_partial;
    }
}
