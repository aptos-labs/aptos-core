

module aptos_framework::move_to_auth {
    use std::signer;

    use aptos_framework::create_signer::create_signer;

    friend aptos_framework::object;

    // without "store" this is equivalent to today
    // with store - this enables new behavior
    struct WriteResourceRef has store, drop, copy {
        addr: address,
    }

    public inline fun move_to_with_ref<T: key>(ref: &WriteResourceRef, value: T) {
        // cannot inline &create_signer(ref.addr)
        // options:
        // - move_to_with_ref should be another instruction
        // - have special handling for visibility
        // - allow move_to arbitrary T inside of move_to_with_ref alone

        move_to<T>(&create_signer(ref.addr), value);
    }

    public fun create_write_ref(owner: &signer): WriteResourceRef {
        // if we add permissions for this, check permissions

        WriteResourceRef { addr: signer::address_of(owner)}
    }

    public(friend) fun create_write_ref_privileged(addr: address): WriteResourceRef {
        WriteResourceRef { addr }
    }
}
