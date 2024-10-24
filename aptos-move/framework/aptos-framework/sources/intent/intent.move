module aptos_framework::intent {
    use std::error;
    use aptos_framework::function_info::{Self, FunctionInfo};
    use aptos_framework::object::{Self, DeleteRef, Object};
    use aptos_framework::timestamp;

    use std::signer;
    use std::string;
    use aptos_std::hot_potato_any::{Self, Any};

    /// The offered intent has expired
    const EINTENT_EXPIRED: u64 = 0;

    /// The registered hook function for consuming resource doesn't match the type requirement.
    const ECONSUMPTION_FUNCTION_TYPE_MISMATCH: u64 = 1;

    /// Only owner can revoke an intent.
    const ENOT_OWNER: u64 = 2;

    struct TradeIntent<Source, phantom Target, Args> has key {
        offered_resource: Source,
        argument: Args,
        self_delete_ref: DeleteRef,
        expiry_time: u64,
        consumption_function: FunctionInfo,
    }

    struct TradeSession<phantom Target, Args> {
        argument: Args,
        consumption_function: FunctionInfo,
    }

    // Core offering logic

    public fun create_intent<Source: store, Target, Args: store + drop>(
        offered_resource: Source,
        argument: Args,
        expiry_time: u64,
        consumption_function: FunctionInfo,
        issuer: address,
    ): Object<TradeIntent<Source, Target, Args>> {
        let constructor_ref = object::create_object(issuer);
        let object_signer = object::generate_signer(&constructor_ref);
        let self_delete_ref = object::generate_delete_ref(&constructor_ref);
        let dispatch_consumption_function_info = function_info::new_function_info_from_address(
            @aptos_framework,
            string::utf8(b"intent"),
            string::utf8(b"dispatch_consumption"),
        );
        // Verify that caller type matches callee type so wrongly typed function cannot be registered.
        assert!(
            function_info::check_dispatch_type_compatibility(
                &dispatch_consumption_function_info,
                &consumption_function,
            ),
            error::invalid_argument(
                ECONSUMPTION_FUNCTION_TYPE_MISMATCH
            )
        );

        move_to<TradeIntent<Source, Target, Args>>(
            &object_signer,
            TradeIntent {
                offered_resource,
                argument,
                expiry_time,
                self_delete_ref,
                consumption_function,
            }
        );
        object::object_from_constructor_ref(&constructor_ref)
    }

    public fun start_intent_session<Source: store, Target, Args: store + drop>(
        intent: Object<TradeIntent<Source, Target, Args>>,
    ): (Source, TradeSession<Target, Args>) acquires TradeIntent {
        let intent_ref = borrow_global<TradeIntent<Source, Target, Args>>(object::object_address(&intent));
        assert!(timestamp::now_seconds() <= intent_ref.expiry_time, error::permission_denied(EINTENT_EXPIRED));

        let TradeIntent {
            offered_resource,
            argument,
            expiry_time: _,
            self_delete_ref,
            consumption_function,
        } = move_from<TradeIntent<Source, Target, Args>>(object::object_address(&intent));

        object::delete(self_delete_ref);

        return (offered_resource, TradeSession {
            argument,
            consumption_function,
        })
    }

    public fun finish_intent_session<Target, Args: store + drop>(
        session: TradeSession<Target, Args>,
        desired_target: Target,
    ) {
        let TradeSession {
            argument,
            consumption_function,
        } = session;

        let p_argument = hot_potato_any::pack(argument);
        let p_target = hot_potato_any::pack(desired_target);

        function_info::load_module_from_function(&consumption_function);
        dispatch_consumption(p_target, p_argument, &consumption_function);
    }

    public fun revoke_intent<Source: store, Target, Args: store + drop>(
        issuer: &signer,
        intent: Object<TradeIntent<Source, Target, Args>>,
    ): Source acquires TradeIntent {
        assert!(object::owner(intent) == signer::address_of(issuer), error::permission_denied(ENOT_OWNER));
        let TradeIntent {
            offered_resource,
            argument: _,
            expiry_time: _,
            self_delete_ref,
            consumption_function: _,
        } = move_from<TradeIntent<Source, Target, Args>>(object::object_address(&intent));

        object::delete(self_delete_ref);
        offered_resource
    }

    native fun dispatch_consumption(target: Any, argument: Any, function: &FunctionInfo);
}
