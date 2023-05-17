module aptoswap::init {
    use std::signer;
    use aptos_framework::object::{Self, ExtendRef};

    const ERR_NOT_APTOSWAP_ADMIN: u64 = 1;
    const ERR_EXTEND_REF_NOT_MATCH: u64 = 2;

    struct PoolManagerExtendRef has key {
        ref: ExtendRef
    }

    /// Creates new object for Aptoswap to be the pool manager object where the lp code will be deployed at.
    /// Can only be executed only from Aptoswap admin account.
    public entry fun initialize_lp_account(
        admin: &signer,
        code_metadata_serialized: vector<u8>,
        code: vector<u8>
    ) {
        assert!(signer::address_of(admin) == @aptoswap, std::error::permission_denied(ERR_NOT_APTOSWAP_ADMIN));

        let contructor_ref = &object::create_named_object(admin, b"aptoswap_pool_manager_seed");
        let obj_signer = object::generate_signer(contructor_ref);

        aptos_framework::code::publish_package_txn(
            &obj_signer,
            code_metadata_serialized,
            vector[code]
        );
        move_to(&obj_signer, PoolManagerExtendRef { ref: object::generate_extend_ref(contructor_ref) });
    }

    /// Destroys temporary storage for resource object extend ref.
    /// It needs for initialization of aptoswap pool manager.
    public fun retrieve_extend_ref(admin: &signer): ExtendRef acquires PoolManagerExtendRef {
        let PoolManagerExtendRef { ref } =
            move_from<PoolManagerExtendRef>(signer::address_of(admin));
        assert!(
            signer::address_of(admin) == object::address_from_extend_ref(&ref),
            std::error::permission_denied(ERR_EXTEND_REF_NOT_MATCH)
        );
        ref
    }
}
