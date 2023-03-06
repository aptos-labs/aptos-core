/// This defines an object-based Royalty. The royalty can be applied to either a collection or a
/// token. Applications should read the royalty from the token, as it will read the appropriate
/// royalty.
module token_objects::royalty {
    use std::option::{Self, Option};

    use aptos_framework::object::{Self, Object};

    friend token_objects::collection;
    friend token_objects::token;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The royalty of a token within this collection -- this optional
    struct Royalty has copy, drop, key {
        numerator: u64,
        denominator: u64,
        /// The recipient of royalty payments. See the `shared_account` for how to handle multiple
        /// creators.
        payee_address: address,
    }

    public(friend) fun init(object_signer: &signer, royalty: Royalty) {
        move_to(object_signer, royalty);
    }

    public fun create(numerator: u64, denominator: u64, payee_address: address): Royalty {
        Royalty { numerator, denominator, payee_address }
    }

    // Accessors
    public fun royalty<T: key>(maybe_royalty: Object<T>): Option<Royalty> acquires Royalty {
        let obj_addr = object::object_address(&maybe_royalty);
        if (exists<Royalty>(obj_addr)) {
            option::some(*borrow_global<Royalty>(obj_addr))
        } else {
            option::none()
        }
    }

    public fun denominator(royalty: &Royalty): u64 {
        royalty.denominator
    }

    public fun numerator(royalty: &Royalty): u64 {
        royalty.numerator
    }

    public fun payee_address(royalty: &Royalty): address {
        royalty.payee_address
    }
}
