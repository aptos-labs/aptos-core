/// This defines an object-based Royalty. The royalty can be applied to either a collection or a
/// token. Applications should read the royalty from the token, as it will read the appropriate
/// royalty.
module token_objects::royalty {
    use std::error;
    use std::option::{Self, Option};

    use aptos_framework::object::{Self, ConstructorRef, ExtendRef, Object};

    // Enforce that the royalty is between 0 and 1
    const EROYALTY_EXCEEDS_MAXIMUM: u64 = 1;
    // Enforce that the denominator of a royalty is not 0
    const EROYALTY_DENOMINATOR_IS_ZERO: u64 = 2;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The royalty of a token within this collection -- this optional
    struct Royalty has copy, drop, key {
        numerator: u64,
        denominator: u64,
        /// The recipient of royalty payments. See the `shared_account` for how to handle multiple
        /// creators.
        payee_address: address,
    }

    struct MutatorRef has drop, store {
        inner: ExtendRef,
    }

    /// Add a royalty, given a ConstructorRef.
    public fun init(ref: &ConstructorRef, royalty: Royalty) {
        let signer = object::generate_signer(ref);
        move_to(&signer, royalty);
    }

    /// Set the royalty if it does not exist, replace it otherwise.
    public fun update(mutator_ref: &MutatorRef, royalty: Royalty) acquires Royalty {
        let addr = object::address_from_extend_ref(&mutator_ref.inner);
        if (exists<Royalty>(addr)) {
            move_from<Royalty>(addr);
        };

        let signer = object::generate_signer_for_extending(&mutator_ref.inner);
        move_to(&signer, royalty);
    }

    public fun create(numerator: u64, denominator: u64, payee_address: address): Royalty {
        assert!(denominator != 0, error::out_of_range(EROYALTY_DENOMINATOR_IS_ZERO));
        assert!(numerator <= denominator, error::out_of_range(EROYALTY_EXCEEDS_MAXIMUM));

        Royalty { numerator, denominator, payee_address }
    }

    public fun generate_mutator_ref(ref: ExtendRef): MutatorRef {
        MutatorRef { inner: ref }
    }

    // Accessors
    public fun get<T: key>(maybe_royalty: Object<T>): Option<Royalty> acquires Royalty {
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

    #[test(creator = @0x123)]
    fun test_none(creator: &signer) acquires Royalty {
        let constructor_ref = object::create_named_object(creator, b"");
        let object = object::object_from_constructor_ref<object::ObjectCore>(&constructor_ref);
        assert!(option::none() == get(object), 0);
    }

    #[test(creator = @0x123)]
    fun test_init_and_update(creator: &signer) acquires Royalty {
        let constructor_ref = object::create_named_object(creator, b"");
        let object = object::object_from_constructor_ref<object::ObjectCore>(&constructor_ref);
        let init_royalty = create(1, 2, @0x123);
        init(&constructor_ref, init_royalty);
        assert!(option::some(init_royalty) == get(object), 0);

        let mutator_ref = generate_mutator_ref(object::generate_extend_ref(&constructor_ref));
        let update_royalty = create(1, 5, @0x123);
        update(&mutator_ref, update_royalty);
        assert!(option::some(update_royalty) == get(object), 1);
    }

    #[test(creator = @0x123)]
    fun test_update_only(creator: &signer) acquires Royalty {
        let constructor_ref = object::create_named_object(creator, b"");
        let object = object::object_from_constructor_ref<object::ObjectCore>(&constructor_ref);
        assert!(option::none() == get(object), 0);

        let mutator_ref = generate_mutator_ref(object::generate_extend_ref(&constructor_ref));
        let update_royalty = create(1, 5, @0x123);
        update(&mutator_ref, update_royalty);
        assert!(option::some(update_royalty) == get(object), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x20001, location = Self)]
    fun test_exceeds_maximum() {
        create(6, 5, @0x1);
    }

    #[test]
    #[expected_failure(abort_code = 0x20002, location = Self)]
    fun test_invalid_denominator() {
        create(6, 0, @0x1);
    }
}
