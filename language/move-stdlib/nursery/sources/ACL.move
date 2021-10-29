/// Access control list (ACL) module. An ACL is a list of account addresses who
/// have the access permission to a certain object.
/// This module uses a `vector` to represent the list, but can be refactored to
/// use a "set" instead when it's available in the language in the future.

module Std::ACL {
    use Std::Vector;
    use Std::Errors;

    /// The ACL already contains the address.
    const ECONTAIN: u64 = 0;
    /// The ACL does not contain the address.
    const ENOT_CONTAIN: u64 = 1;

    struct ACL has store, drop, copy {
        list: vector<address>
    }

    /// Return an empty ACL.
    public fun empty(): ACL {
        ACL{ list: Vector::empty<address>() }
    }

    /// Add the address to the ACL.
    public fun add(acl: &mut ACL, addr: address) {
        assert!(!Vector::contains(&mut acl.list, &addr), Errors::invalid_argument(ECONTAIN));
        Vector::push_back(&mut acl.list, addr);
    }

    /// Remove the address from the ACL.
    public fun remove(acl: &mut ACL, addr: address) {
        let (found, index) = Vector::index_of(&mut acl.list, &addr);
        assert!(found, Errors::invalid_argument(ENOT_CONTAIN));
        Vector::remove(&mut acl.list, index);
    }

    /// Return true iff the ACL contains the address.
    public fun contains(acl: &ACL, addr: address): bool {
        Vector::contains(&acl.list, &addr)
    }

    /// assert! that the ACL has the address.
    public fun assert_contains(acl: &ACL, addr: address) {
        assert!(contains(acl, addr), Errors::invalid_argument(ENOT_CONTAIN));
    }
}
