module 0x42::create_signer {
    friend 0x42::object;
    public(friend) native fun create_signer(addr: address): signer;
}

module 0x42::signer {
    native public fun borrow_address(s: &signer): &address;

    // Copies the address of the signer
    public fun address_of(s: &signer): address {
        *borrow_address(s)
    }


}

module 0x42::event {
    public fun emit<T: store + drop>(_msg: T) {
        // write_module_event_to_store<T>(msg);
    }

    public fun emit_event<T: drop + store>(_handle_ref: &mut EventHandle<T>, _msg: T) {
        // write_to_event_store<T>(bcs::to_bytes(&handle_ref.guid), handle_ref.counter, msg);
    }

    struct EventHandle<phantom T: drop + store> has store {
        /// Total number of events emitted to this event stream.
        counter: u64,
        /// A globally unique ID for this event stream.
        guid: u64,
    }
}

module 0x42::error {
    public fun permission_denied(i: u64): u64 {
        i
    }
}

module 0x42::object {
    use 0x42::create_signer::create_signer;
    use 0x42::signer;
    use 0x42::event;

    const ENOT_OBJECT_OWNER: u64 = 4;
    const EOBJECT_DOES_NOT_EXIST: u64 = 5;
    const BURN_ADDRESS: address = @0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;

    struct Transfer has drop, store {
        object: address,
        from: address,
        to: address,
    }

    struct TransferEvent has drop, store {
        object: address,
        from: address,
        to: address,
    }

    struct TombStone has key {
        /// Track the previous owner before the object is burnt so they can reclaim later if so desired.
        original_owner: address,
    }

    struct ObjectCore has key {
        guid_creation_num: u64,
        owner: address,
        transfer_events: event::EventHandle<TransferEvent>,
    }

    struct Object<phantom T> has copy, drop, store {
        inner: address,
    }

    inline fun transfer_raw_inner(object: address, to: address, a1: bool) acquires ObjectCore {
        let object_core = borrow_global_mut<ObjectCore>(object);
        if (object_core.owner != to) {
            if (a1) {
                event::emit(
                    Transfer {
                        object,
                        from: object_core.owner,
                        to,
                    },
                );
            };
            event::emit_event(
                &mut object_core.transfer_events,
                TransferEvent {
                    object,
                    from: object_core.owner,
                    to,
                },
            );
            object_core.owner = to;
        };
    }

    public entry fun burn<T: key>(owner: &signer, object: Object<T>, a1: bool) acquires ObjectCore {
        let original_owner = signer::address_of(owner);
        assert!(is_owner(object, original_owner), ENOT_OBJECT_OWNER);
        let object_addr = object.inner;
        move_to(&create_signer(object_addr), TombStone { original_owner });
        transfer_raw_inner(object_addr, BURN_ADDRESS, a1);
    }

    public fun owner<T: key>(object: Object<T>): address acquires ObjectCore {
        assert!(
            exists<ObjectCore>(object.inner),
            EOBJECT_DOES_NOT_EXIST,
        );
        borrow_global<ObjectCore>(object.inner).owner
    }

    public fun is_owner<T: key>(object: Object<T>, owner: address): bool acquires ObjectCore {
        owner(object) == owner
    }
}
