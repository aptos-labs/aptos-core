module aptos_framework::intent {
    use std::error;
    use std::signer;
    use aptos_framework::object::{Self, DeleteRef, Object};
    use aptos_framework::timestamp;
    use aptos_framework::type_info::{Self, TypeInfo};

    /// The offered intent has expired
    const EINTENT_EXPIRED: u64 = 0;

    /// The registered hook function for consuming resource doesn't match the type requirement.
    const ECONSUMPTION_FUNCTION_TYPE_MISMATCH: u64 = 1;

    /// Only owner can revoke an intent.
    const ENOT_OWNER: u64 = 2;

    /// Provided wrong witness to complete intent.
    const EINVALID_WITNESS: u64 = 3;

    struct TradeIntent<Source, Args> has key {
        offered_resource: Source,
        argument: Args,
        self_delete_ref: DeleteRef,
        expiry_time: u64,
        witness_type: TypeInfo,
    }

    struct TradeSession<Args> {
        argument: Args,
        witness_type: TypeInfo,
    }

    // Core offering logic

    public fun create_intent<Source: store, Args: store + drop, Witness: drop>(
        offered_resource: Source,
        argument: Args,
        expiry_time: u64,
        issuer: address,
        _witness: Witness,
    ): Object<TradeIntent<Source, Args>> {
        let constructor_ref = object::create_object(issuer);
        let object_signer = object::generate_signer(&constructor_ref);
        let self_delete_ref = object::generate_delete_ref(&constructor_ref);

        move_to<TradeIntent<Source, Args>>(
            &object_signer,
            TradeIntent {
                offered_resource,
                argument,
                expiry_time,
                self_delete_ref,
                witness_type: type_info::type_of<Witness>(),
            }
        );
        object::object_from_constructor_ref(&constructor_ref)
    }

    public fun start_intent_session<Source: store, Args: store + drop>(
        intent: Object<TradeIntent<Source, Args>>,
    ): (Source, TradeSession<Args>) acquires TradeIntent {
        let intent_ref = borrow_global<TradeIntent<Source, Args>>(object::object_address(&intent));
        assert!(timestamp::now_seconds() <= intent_ref.expiry_time, error::permission_denied(EINTENT_EXPIRED));

        let TradeIntent {
            offered_resource,
            argument,
            expiry_time: _,
            self_delete_ref,
            witness_type,
        } = move_from<TradeIntent<Source, Args>>(object::object_address(&intent));

        object::delete(self_delete_ref);

        return (offered_resource, TradeSession {
            argument,
            witness_type,
        })
    }

    public fun get_argument<Args>(session: &TradeSession<Args>): &Args {
        &session.argument
    }

    public fun finish_intent_session<Witness: drop, Args: store + drop>(
        session: TradeSession<Args>,
        _witness: Witness,
    ) {
        let TradeSession {
            argument:_ ,
            witness_type,
        } = session;

        assert!(type_info::type_of<Witness>() == witness_type, error::permission_denied(EINVALID_WITNESS));
    }

    public fun revoke_intent<Source: store, Args: store + drop>(
        issuer: &signer,
        intent: Object<TradeIntent<Source, Args>>,
    ): Source acquires TradeIntent {
        assert!(object::owner(intent) == signer::address_of(issuer), error::permission_denied(ENOT_OWNER));
        let TradeIntent {
            offered_resource,
            argument: _,
            expiry_time: _,
            self_delete_ref,
            witness_type: _,
        } = move_from<TradeIntent<Source, Args>>(object::object_address(&intent));

        object::delete(self_delete_ref);
        offered_resource
    }
}
