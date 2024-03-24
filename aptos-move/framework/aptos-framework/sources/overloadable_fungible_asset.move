/// This defines the fungible asset module that can issue fungible asset of any `Metadata` object. The
/// metadata object can be any object that equipped with `Metadata` resource.
///
/// The overloadable_fungible_asset wraps the existing fungible_asset module and adds the ability for token issuer
/// to customize the logic for withdraw and deposit operations. For example:
///
/// - Deflation token: a fixed percentage of token will be destructed upon transfer.
/// - Transfer allowlist: token can only be transfered to addresses in the allow list.
/// - Predicated transfer: transfer can only happen when some certain predicate has been met.
/// - Loyalty token: a fixed loyalty will be paid to a designated address when a fungible asset transfer happens
///
/// See AIP-73 for further discussion
///
module aptos_framework::overloadable_fungible_asset {
    use aptos_framework::fungible_asset::{Self, FungibleAsset, TransferRef};
    use aptos_framework::function_info::{Self, FunctionInfo};
    use aptos_framework::object::{Self, ConstructorRef, Object};

    use std::error;
    use std::string;
    use std::signer;

    /// Provided withdraw function type doesn't meet the signature requirement.
    const EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH: u64 = 1;
    /// Provided deposit function type doesn't meet the signature requirement.
    const EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH: u64 = 2;
    /// Calling overloadable api on non-overloadable fungible asset store.
    const EFUNCTION_STORE_NOT_FOUND: u64 = 3;
    /// Recipient is not getting the guaranteed value;
    const EAMOUNT_MISMATCH: u64 = 4;
    /// Trying to register overload functions to fungible asset that has already been initialized with custom transfer function.
    const EALREADY_REGISTERED: u64 = 4;
    /// Fungibility is only available for non-deletable objects.
    const EOBJECT_IS_DELETABLE: u64 = 18;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct OverloadFunctionStore has key {
		withdraw_function: FunctionInfo,
		deposit_function: FunctionInfo,
        transfer_ref: TransferRef,
    }

    /// Create a fungible asset store whose transfer rule would be overloaded by the provided function.
    public fun register_overload_functions(
        constructor_ref: &ConstructorRef,
        withdraw_function: FunctionInfo,
		deposit_function: FunctionInfo,
    ) {
        let dispatcher_withdraw_function_info = function_info::new_function_info(
	        @aptos_framework,
            string::utf8(b"overloadable_fungible_asset"),
            string::utf8(b"dispatchable_withdraw"),
        );
        // Verify that caller type matches callee type so wrongly typed function cannot be registered.
        assert!(function_info::check_dispatch_type_compatibility(
            &dispatcher_withdraw_function_info,
            &withdraw_function
        ), error::invalid_argument(EWITHDRAW_FUNCTION_SIGNATURE_MISMATCH));

        let dispatcher_deposit_function_info = function_info::new_function_info(
	        @aptos_framework,
            string::utf8(b"overloadable_fungible_asset"),
            string::utf8(b"dispatchable_deposit"),
        );
        // Verify that caller type matches callee type so wrongly typed function cannot be registered.
        assert!(function_info::check_dispatch_type_compatibility(
            &dispatcher_deposit_function_info,
            &deposit_function
        ), error::invalid_argument(EDEPOSIT_FUNCTION_SIGNATURE_MISMATCH));

        assert!(!object::can_generate_delete_ref(constructor_ref), error::invalid_argument(EOBJECT_IS_DELETABLE));
        assert!(!exists<OverloadFunctionStore>(object::address_from_constructor_ref(constructor_ref)), error::already_exists(EALREADY_REGISTERED));

        // Freeze the FungibleStore to force usign the new overloaded api.
        let extend_ref = object::generate_extend_ref(constructor_ref);
        fungible_asset::set_global_frozen_flag(&extend_ref, true);

        let store_obj = &object::generate_signer(constructor_ref);

        // Store the overload function hook.
        move_to<OverloadFunctionStore>(store_obj, OverloadFunctionStore {
            withdraw_function,
		    deposit_function,
            transfer_ref: fungible_asset::generate_transfer_ref(constructor_ref),
        });
    }

    /// Withdraw `amount` of the fungible asset from `store` by the owner.
    ///
    /// The semantics of deposit will be governed by the function specified in OverloadFunctionStore.
    public fun withdraw<T: key>(
        owner: &signer,
        store: Object<T>,
        amount: u64,
    ): FungibleAsset acquires OverloadFunctionStore {
        let metadata_addr = object::object_address(&fungible_asset::store_metadata(store));
        let owner_address = signer::address_of(owner);
        assert!(exists<OverloadFunctionStore>(metadata_addr), error::not_found(EFUNCTION_STORE_NOT_FOUND));
        let overloadable_store = borrow_global<OverloadFunctionStore>(metadata_addr);
        dispatchable_withdraw(
            owner_address,
            store,
            amount,
            &overloadable_store.transfer_ref,
            &overloadable_store.withdraw_function,
        )
    }

    /// Deposit `amount` of the fungible asset to `store`.
    ///
    /// The semantics of deposit will be governed by the function specified in OverloadFunctionStore.
    public fun deposit<T: key>(
        store: Object<T>,
        fa: FungibleAsset
    ) acquires OverloadFunctionStore {
        let metadata_addr = object::object_address(&fungible_asset::store_metadata(store));
        assert!(exists<OverloadFunctionStore>(metadata_addr), error::not_found(EFUNCTION_STORE_NOT_FOUND));
        let overloadable_store = borrow_global<OverloadFunctionStore>(metadata_addr);
        dispatchable_deposit(
            store,
            fa,
            &overloadable_store.transfer_ref,
            &overloadable_store.deposit_function,
        );
    }

    /// A transfer with a fixed amount debited from the sender
    public fun transfer_fixed_send<T: key>(
        _sender: &signer,
        from: Object<T>,
        to: Object<T>,
        send_amount: u64,
    ) acquires OverloadFunctionStore {
        let store_address = object::object_address(&from);
        assert!(exists<OverloadFunctionStore>(store_address), error::not_found(EFUNCTION_STORE_NOT_FOUND));
        let overloadable_store = borrow_global<OverloadFunctionStore>(store_address);
        let fa = fungible_asset::withdraw_with_ref(&overloadable_store.transfer_ref, from, send_amount);
        deposit(to, fa);
    }

    /// A transfer with a fixed amount credited to the recipient
    public fun transfer_fixed_receive<T: key>(
        sender: &signer,
        from: Object<T>,
        to: Object<T>,
        receive_amount: u64,
    ) acquires OverloadFunctionStore {
        let fa = withdraw(sender, from, receive_amount);
        let store_address = object::object_address(&from);
        let overloadable_store = borrow_global<OverloadFunctionStore>(store_address);
        assert!(fungible_asset::amount(&fa) == receive_amount, error::aborted(EAMOUNT_MISMATCH));
        fungible_asset::deposit_with_ref(&overloadable_store.transfer_ref, to, fa);
    }

    native fun dispatchable_withdraw<T: key>(
        owner: address,
        store: Object<T>,
        amount: u64,
        transfer_ref: &TransferRef,
        function: &FunctionInfo,
    ): FungibleAsset;

    native fun dispatchable_deposit<T: key>(
        store: Object<T>,
        fa: FungibleAsset,
        transfer_ref: &TransferRef,
        function: &FunctionInfo,
    );
}
