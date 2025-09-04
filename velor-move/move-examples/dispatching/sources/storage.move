/// The storage module stores all the state associated with the dispatch service.
module dispatching::storage {
    use std::option;
    use std::string;

    use velor_std::table::{Self, Table};
    use velor_std::type_info::{Self, TypeInfo};

    use velor_framework::dispatchable_fungible_asset;
    use velor_framework::function_info::FunctionInfo;
    use velor_framework::fungible_asset::{Self, Metadata};
    use velor_framework::object::{Self, ExtendRef, Object};

    friend dispatching::engine;

    struct Dispatcher has key {
        /// Tracks the input type to the dispatch handler.
        dispatcher: Table<TypeInfo, Object<Metadata>>,
        /// Used to store temporary data for dispatching.
        obj_ref: ExtendRef,
    }

    /// Store the data to dispatch here.
    struct Storage<phantom T> has drop, key {
        data: vector<u8>,
    }

    /// Register a `T` to callback. Providing an instance of `T` guarantees that only the
    /// originating module can call `register` for that type.
    public fun register<T: drop>(callback: FunctionInfo, _proof: T) acquires Dispatcher {
        let typename = type_info::type_name<T>();
        let constructor_ref = object::create_named_object(&storage_signer(), *string::bytes(&typename));
        let metadata = fungible_asset::add_fungibility(
            &constructor_ref,
            option::none(),
            typename,
            string::utf8(b"dis"),
            0,
            string::utf8(b""),
            string::utf8(b""),
        );
        dispatchable_fungible_asset::register_derive_supply_dispatch_function(
            &constructor_ref,
            option::some(callback),
        );

        let dispatcher = borrow_global_mut<Dispatcher>(@dispatching);
        table::add(&mut dispatcher.dispatcher, type_info::type_of<T>(), metadata);
    }

    /// Insert into this module as the callback needs to retrieve and avoid a cyclical dependency:
    /// engine -> storage and then engine -> callback -> storage
    public(friend) fun insert<T>(data: vector<u8>): Object<Metadata> acquires Dispatcher {
        move_to(&storage_signer(), Storage<T> { data });

        let typeinfo = type_info::type_of<T>();
        let dispatcher = borrow_global<Dispatcher>(@dispatching);
        *table::borrow(&dispatcher.dispatcher, typeinfo)
    }

    /// Second half of the process for retrieving. This happens outside engine to prevent the
    /// cyclical dependency.
    public fun retrieve<T: drop>(_proof: T): vector<u8> acquires Dispatcher, Storage {
        move_from<Storage<T>>(storage_address()).data
    }

    /// Prepares the dispatch table.
    fun init_module(publisher: &signer) {
        let constructor_ref = object::create_object(@dispatching);

        move_to(
            publisher,
            Dispatcher {
                dispatcher: table::new(),
                obj_ref: object::generate_extend_ref(&constructor_ref),
            }
        );
    }

    inline fun storage_address(): address acquires Dispatcher {
        object::address_from_extend_ref(&borrow_global<Dispatcher>(@dispatching).obj_ref)
    }

    inline fun storage_signer(): signer acquires Dispatcher {
        object::generate_signer_for_extending(&borrow_global<Dispatcher>(@dispatching).obj_ref)
    }

    #[test_only]
    public fun init_module_for_testing(publisher: &signer) {
        init_module(publisher);
    }
}
