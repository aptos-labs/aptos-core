module inscriptions::inscriptions {
    use std::error;

    use aptos_framework::event;
    use aptos_framework::object::{Self, ConstructorRef, Object};

    use aptos_token_objects::token;

    /// The token does not exist
    const ETOKEN_DOES_NOT_EXIST: u64 = 1;
    /// The inscription does not exist
    const EINSCRIPTION_DOES_NOT_EXIST: u64 = 2;

    #[event]
    struct InscriptionData has drop, store {
        inscription_id: u64,
        data: vector<u8>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct InscriptionMetadata has key {
        inscription_id: u64,
    }

    #[resource_group(scope = module_)]
    struct InscriptionStateGroup { }

    #[resource_group_member(group = inscriptions::inscriptions::InscriptionStateGroup)]
    struct InscriptionState has key {
        next_inscription_id: u64,
    }

    public fun add_inscription(
        constructor_ref: &ConstructorRef,
        data: vector<u8>,
    ): u64 acquires InscriptionState {
        assert!(
            object::object_exists<token::Token>(object::address_from_constructor_ref(constructor_ref)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        let object_signer = object::generate_signer(constructor_ref);

        let inscription_id = get_next_inscription_id();
        let inscription_metadata = InscriptionMetadata { inscription_id };
        move_to(&object_signer, inscription_metadata);

        let inscription_data = InscriptionData {
            inscription_id,
            data,
        };
        event::emit(inscription_data);

        inscription_id
    }

    fun get_next_inscription_id(): u64 acquires InscriptionState {
        let inscription_state = borrow_global_mut<InscriptionState>(@inscriptions);
        let inscription_id = inscription_state.next_inscription_id;
        inscription_state.next_inscription_id = inscription_state.next_inscription_id + 1;
        inscription_id
    }

    public fun is_inscription<T: key>(object: Object<T>): bool {
        exists<InscriptionMetadata>(object::object_address(&object))
    }

    public fun inscription_id<T: key>(object: Object<T>): u64 acquires InscriptionMetadata {
        let addr = object::object_address(&object);
        assert!(
            exists<InscriptionMetadata>(addr),
            error::not_found(EINSCRIPTION_DOES_NOT_EXIST),
        );
        borrow_global<InscriptionMetadata>(addr).inscription_id
    }

    fun init_module(deployer: &signer) {
        let inscription_state = InscriptionState { next_inscription_id: 0 };
        move_to(deployer, inscription_state);
    }

    #[test_only]
    public fun init_for_test(deployer: &signer) {
        init_module(deployer);
    }

    #[test_only]
    use std::option;
    #[test_only]
    use std::signer;
    #[test_only]
    use std::string::{Self, String};
    #[test_only]
    use aptos_token_objects::collection;
    #[test_only]
    use aptos_token_objects::royalty;

    #[test(creator = @0x123, deployer = @inscriptions)]
    fun test_create(creator: &signer, deployer: &signer) acquires InscriptionMetadata, InscriptionState {
        let collection = string::utf8(b"collection");
        let inscription_0 = b"00000000";
        let inscription_1 = b"00000000";

        init_for_test(deployer);
        let _collection_ref = create_collection_helper(creator, collection, 10);

        let token_0_ref = create_token_helper(creator, collection, string::utf8(b"0"));
        let token_0_obj = object::object_from_constructor_ref<token::Token>(&token_0_ref);
        assert!(!is_inscription(token_0_obj), 2);
        add_inscription(&token_0_ref, inscription_0);
        assert!(event::was_event_emitted(&InscriptionData { inscription_id: 0, data: inscription_0 }), 0);
        assert!(!event::was_event_emitted(&InscriptionData { inscription_id: 1, data: inscription_1 }), 1);
        assert!(is_inscription(token_0_obj), 2);
        assert!(inscription_id(token_0_obj) == 0, 2);

        let token_1_ref = create_token_helper(creator, collection, string::utf8(b"1"));
        let token_1_obj = object::object_from_constructor_ref<token::Token>(&token_1_ref);
        add_inscription(&token_1_ref, inscription_1);
        assert!(event::was_event_emitted(&InscriptionData { inscription_id: 1, data: inscription_1 }), 1);
        assert!(is_inscription(token_1_obj), 2);
        assert!(inscription_id(token_1_obj) == 1, 2);
    }

    #[test_only]
    fun create_collection_helper(creator: &signer, collection_name: String, max_supply: u64): object::ExtendRef {
        let constructor_ref = collection::create_fixed_collection(
            creator,
            string::utf8(b"collection description"),
            max_supply,
            collection_name,
            option::none(),
            string::utf8(b"collection uri"),
        );
        object::generate_extend_ref(&constructor_ref)
    }

    #[test_only]
    fun create_token_helper(creator: &signer, collection_name: String, token_name: String): ConstructorRef {
        token::create_named_token(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::some(royalty::create(25, 10000, signer::address_of(creator))),
            string::utf8(b"uri"),
        )
    }
}
