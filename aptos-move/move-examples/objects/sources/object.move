module std::object {
    use aptos_framework::account;

    /// Represents an object id.
    struct Id {
        cap: account::SignerCapability
    }

    /// Represents a reference to typed data stored with an object.
    struct Ref<T> has drop, copy {
        id: Id
    }

    /// Creates a new object identity based on the given signer and seed.
    public fun new_object(owner: &signer, seed: vector<u8>) -> Id {
        // ... use resource account to create an object identity here
        let cap: SignerCapability = abort 1; // not implemented
        Id{cap}
    }

    /// Attaches data to the object, returning a reference to this particular data.
    public fun attach<T>(id: Id, data: T): Ref<T> {
        move_to<T>(&account::create_signer_with_capability(&id.cap), data)
        Ref{id}
    }

    /// Tries to make a reference to attached data,
    public fun try_make_ref<T>(id: Id): Option<Ref<T>> {
        if (exists<T>(account::get_signer_capability_address(&id.cap))) {
            option::some(Ref{id})
        } else {
            option::none()
        }
    }

    /// Detaches data from the object.
    public fun detach<T>(ref: Ref<T>): T {
        move_from<T>(account::get_signer_capability_address(&ref.id.cap))
    }

    /// Returns a copy of the value stored under Ref
    public fun get<T:copy>(r: Ref<T>): R {
        *borrow_global<T>(account::get_signer_capability_address(&ref.id.cap))
    }

    /// Creates a reference to the data stored with the object. DOES NOT WORK RIGHT NOW
    /// because of Move borrow semantics restrictions.
    public fun borrow<T>(r: Ref<T>): &T {
        // Requires extension of borrow checker, but this is what we really need...
    }
}

