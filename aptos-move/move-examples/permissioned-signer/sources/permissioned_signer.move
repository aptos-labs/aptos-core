/// A _permissioned signer_ consists of a pair of the original signer and a generated
/// signer which is used store information about associated permissions.
///
/// A permissioned signer behaves compatible with the original signer as it comes to `move_to`, `address_of`, and
/// existing basic signer functionality. However, the permissions can be queried to assert additional
/// restrictions on the use of the signer.
///
/// A client which is interested in restricting access granted via a signer can create a permissioned signer
/// and pass on to other existing code without changes to existing APIs. Core functions in the framework, for
/// example account functions, can then assert availability of permissions, effectively restricting
/// existing code in a compatible way.
///
/// After introducing the core functionality, examples are provided for withdraw limit on accounts, and
/// for blind signing.
module permissioned_signer::permissioned_signer {

    use std::signer::address_of;

    // =====================================================================================================
    // Native Functions

    /// Creates a permissioned signer from an existing universal signer. The function aborts if the
    /// given signer is already a permissioned signer.
    ///
    /// The implementation of this function requires to extend the value representation for signers in the VM.
    public native fun create_permissioned_signer(master: &signer): signer;

    /// Check whether this is a permissioned signer.
    public(package) native fun is_permissioned_signer(s: &signer): bool;

    /// Return the signer used for storing permissions. Aborts if not a permissioned signer.
    public(package) native fun permission_signer(permissioned: &signer): &signer;

    // =====================================================================================================
    // Permission Management

    /// Storage for a permission `Perm`.
    struct StoredPerm<phantom Perm> has key, drop, copy {
        /// A capacity associated with the permission. For example, `StoredPerm<WidthDrawPerm>{capacity: limit}`.
        capacity: u256
    }

    /// Authorizes `permissioned` with the given permission. This requires to have access to the `master`
    /// signer.
    public fun authorize<Perm>(permissioned: &signer, capacity: u256, master: &signer) {
        assert!(
            is_permissioned_signer(permissioned) &&
            !is_permissioned_signer(master) &&
            address_of(master) == address_of(permissioned)
        );
        move_to(permission_signer(permissioned), StoredPerm<Perm>{capacity});
    }

    /// Asserts that the given signer has permission `Perm`, and the capacity
    /// to handle `weight`, which will be subtracted from capacity.
    public fun assert_permission<Perm>(s: &signer, weight: u256) acquires StoredPerm {
        if (!is_permissioned_signer(s)) {
            // master signer has all permissions
        };
        let addr = address_of(permission_signer(s));
        assert!(exists<StoredPerm<Perm>>(addr));
        let perm = &mut StoredPerm<Perm>[addr];
        assert!(perm.capacity >= weight);
        perm.capacity = perm.capacity - weight;
    }

    public fun capacity<Perm>(s: &signer): u256 acquires StoredPerm {
        StoredPerm<Perm>[address_of(permission_signer(s))].capacity
    }
}

/// Example how an account can user permissioned signers to restrict access.
module permissioned_signer::permissioned_account_example {
    use std::signer::address_of;
    use permissioned_signer::permissioned_signer::{assert_permission, authorize};

    struct Account has key {
        balance: u128
    }

    /// The withdraw permission type tag
    struct WithdrawPerm;

    public fun withdraw(s: &signer, amount: u128) acquires Account {
        // If `s` is a general signer, this will succeed, otherwise WithdrawPerm need to be present
        // and have capacity.
        assert_permission<WithdrawPerm>(s, (amount as u256));
        Account[address_of(s)].balance = Account[address_of(s)].balance - amount
    }

    public fun authorize_account_withdraw(permissioned: &signer, limit: u128, master: &signer) {
        authorize<WithdrawPerm>(permissioned, (limit as u256), master)
    }
}

/// Entry example for permissioned signers
module permissioned_signer::permissioned_entry_example {
    use permissioned_signer::permissioned_account_example;
    use permissioned_signer::permissioned_signer::create_permissioned_signer;

    entry fun delegate_to_contract(s: &signer, account_limit: u128) {
        let permissioned = &create_permissioned_signer(s);
        permissioned_account_example::authorize_account_withdraw(permissioned, account_limit, s);

        // call_the_other_contract(permissioned)
    }
}

/// Example how we can implement delegated blind signing.
module permissioned_signer::permissioned_blind_signing_example {
    use std::signer::address_of;
    use aptos_framework::timestamp;
    use permissioned_signer::permissioned_signer::{authorize, assert_permission, capacity};

    /// Permission for storing a signer for blind signing.
    struct BlindSigningPerm;

    /// Authorize the given permissioned signer to be stored for blind signing
    /// for the given number of seconds, starting from now.
    public fun authorize_blind_signing(permissioned: &signer, seconds: u64, master: &signer) {
        authorize<BlindSigningPerm>(permissioned, (timestamp::now_seconds() + seconds as u256), master)
    }

    struct BlindSigner has key {
        // This probably should be a table, as one account can own multiple blind signers.
        repr: vector<u8>
    }

    public fun save_for_blind_signing(owner: &signer, permissioned: &signer) {
        assert_permission<BlindSigningPerm>(permissioned, 0);
        move_to(owner, BlindSigner {repr: permissioned_signer_to_bcs(permissioned)})
    }

    public fun restore_for_blind_signer(owner: &signer): signer acquires BlindSigner {
        let permissioned = bcs_to_permissioned_signer(&BlindSigner[address_of(owner)].repr);
        let expiration_time = (capacity<BlindSigningPerm>(&permissioned) as u64);
        assert!(expiration_time < timestamp::now_seconds());
        permissioned
    }

    /// Private native functions to convert permissioned signer to bytes and back. Abort if the
    /// passed signer is not permissioned.
    native fun permissioned_signer_to_bcs(s: &signer): vector<u8>;
    native fun bcs_to_permissioned_signer(v: &vector<u8>): signer;

}
