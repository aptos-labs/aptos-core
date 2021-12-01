module ExperimentalFramework::Roles {
    use CoreFramework::SystemAddresses;
    use CoreFramework::DiemTimestamp;
    use Std::Errors;
    use Std::Signer;

    friend ExperimentalFramework::ExperimentalAccount;

    /// A `RoleId` resource was in an unexpected state
    const EROLE_ID: u64 = 0;
    /// The signer didn't have the required Diem Root role
    const EDIEM_ROOT: u64 = 1;
    /// The signer didn't have the required Validator role
    const EVALIDATOR: u64 = 7;
    /// The signer didn't have the required Validator Operator role
    const EVALIDATOR_OPERATOR: u64 = 8;

    ///////////////////////////////////////////////////////////////////////////
    // Role ID constants
    ///////////////////////////////////////////////////////////////////////////

    const DIEM_ROOT_ROLE_ID: u64 = 0;
    const VALIDATOR_ROLE_ID: u64 = 3;
    const VALIDATOR_OPERATOR_ROLE_ID: u64 = 4;
    const NO_ROLE_ID: u64 = 100;

    /// The roleId contains the role id for the account. This is only moved
    /// to an account as a top-level resource, and is otherwise immovable.
    struct RoleId has key {
        role_id: u64,
    }

    // =============
    // Role Granting

    /// Publishes diem root role. Granted only in genesis.
    public(friend) fun grant_diem_root_role(
        dr_account: &signer,
    ) {
        DiemTimestamp::assert_genesis();
        // Checks actual Diem root because Diem root role is not set
        // until next line of code.
        SystemAddresses::assert_core_resource(dr_account);
        // Grant the role to the diem root account
        grant_role(dr_account, DIEM_ROOT_ROLE_ID);
    }

    /// Publish a Validator `RoleId` under `new_account`.
    /// The `creating_account` must be diem root.
    public(friend) fun new_validator_role(
        creating_account: &signer,
        new_account: &signer
    ) acquires RoleId {
        assert_diem_root(creating_account);
        grant_role(new_account, VALIDATOR_ROLE_ID);
    }

    /// Publish a ValidatorOperator `RoleId` under `new_account`.
    /// The `creating_account` must be DiemRoot
    public(friend) fun new_validator_operator_role(
        creating_account: &signer,
        new_account: &signer,
    ) acquires RoleId {
        assert_diem_root(creating_account);
        grant_role(new_account, VALIDATOR_OPERATOR_ROLE_ID);
    }

    /// Helper function to grant a role.
    fun grant_role(account: &signer, role_id: u64) {
        assert!(!exists<RoleId>(Signer::address_of(account)), Errors::already_published(EROLE_ID));
        move_to(account, RoleId { role_id });
    }

    // =============
    // Role Checking

    fun has_role(account: &signer, role_id: u64): bool acquires RoleId {
        get_role_id(Signer::address_of(account)) == role_id
    }

    public fun has_diem_root_role(account: &signer): bool acquires RoleId {
        has_role(account, DIEM_ROOT_ROLE_ID)
    }

    public fun has_validator_role(account: &signer): bool acquires RoleId {
        has_role(account, VALIDATOR_ROLE_ID)
    }

    public fun has_validator_operator_role(account: &signer): bool acquires RoleId {
        has_role(account, VALIDATOR_OPERATOR_ROLE_ID)
    }

    public fun get_role_id(addr: address): u64 acquires RoleId {
        if (exists<RoleId>(addr)) {
            borrow_global<RoleId>(addr).role_id
        } else {
            NO_ROLE_ID
        }
    }

    // ===============
    // Role Assertions

    /// Assert that the account is diem root.
    public fun assert_diem_root(account: &signer) acquires RoleId {
        SystemAddresses::assert_core_resource(account);
        let addr = Signer::address_of(account);
        assert!(exists<RoleId>(addr), Errors::not_published(EROLE_ID));
        assert!(borrow_global<RoleId>(addr).role_id == DIEM_ROOT_ROLE_ID, Errors::requires_role(EDIEM_ROOT));
    }

    /// Assert that the account has the validator role.
    public fun assert_validator(validator_account: &signer) acquires RoleId {
        let validator_addr = Signer::address_of(validator_account);
        assert!(exists<RoleId>(validator_addr), Errors::not_published(EROLE_ID));
        assert!(
            borrow_global<RoleId>(validator_addr).role_id == VALIDATOR_ROLE_ID,
            Errors::requires_role(EVALIDATOR)
        )
    }

    /// Assert that the account has the validator operator role.
    public fun assert_validator_operator(validator_operator_account: &signer) acquires RoleId {
        let validator_operator_addr = Signer::address_of(validator_operator_account);
        assert!(exists<RoleId>(validator_operator_addr), Errors::not_published(EROLE_ID));
        assert!(
            borrow_global<RoleId>(validator_operator_addr).role_id == VALIDATOR_OPERATOR_ROLE_ID,
            Errors::requires_role(EVALIDATOR_OPERATOR)
        )
    }
}
