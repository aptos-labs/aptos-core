module dispatching_check_signature::storage {
    use aptos_framework::table::{Self, Table};
    use aptos_framework::fungible_asset::{Metadata};
    use aptos_framework::object::{Self, ExtendRef, Object};

    friend dispatching_check_signature::check_signature;

    const E_DISPATCHER_NOT_FOUND: u64 = 1;
    
    /// The dispatcher table to store the metadata of the dispatcher 
    /// and the data associated with the module address.
    struct Dispatcher has key {
        /// The dispatcher table to store the metadata of the dispatcher.
        dispatcher: Table<address, Object<Metadata>>,
        /// The data table to store the data associated with the module address.
        data: Table<address, vector<u8>>,
        /// The object reference of the dispatcher.
        obj_ref: ExtendRef,
    }

    fun init_module(publisher: &signer) {
      let constructor_ref = object::create_object(@dispatching_check_signature);
      
      move_to(publisher, Dispatcher {
        dispatcher: table::new(),
        data: table::new(),
        obj_ref: object::generate_extend_ref(&constructor_ref),
      });
    }

    /// Retrieves the data associated with the given module address.
    /// This function call by the outside module.
    public fun retrieve(module_address: address): vector<u8> acquires Dispatcher {
        let dispatcher = borrow_global<Dispatcher>(@dispatching_check_signature);
        assert!(table::contains(&dispatcher.data, module_address), E_DISPATCHER_NOT_FOUND);
        *table::borrow(&dispatcher.data, module_address)
    }

    /// Inserts the data associated with the given module address.
    /// This function only call by the dispatcher.
    public(friend) fun insert(module_address: address, data: vector<u8>) acquires Dispatcher {
        let dispatcher = borrow_global_mut<Dispatcher>(@dispatching_check_signature);
        table::upsert(&mut dispatcher.data, module_address, data);
    }

    /// Sets the metadata of the dispatcher.
    /// This function only call by the dispatcher.
    public(friend) fun set_dispatcher_metadata(module_address: address, metadata: Object<Metadata>) acquires Dispatcher {
        let dispatcher = borrow_global_mut<Dispatcher>(@dispatching_check_signature);
        table::add(&mut dispatcher.dispatcher, module_address, metadata);
    }

    #[view]
    public(friend) fun get_signer(): signer acquires Dispatcher {
        let dispatcher = borrow_global<Dispatcher>(@dispatching_check_signature);
        object::generate_signer_for_extending(&dispatcher.obj_ref)
    }

    #[view]
    public fun dispatcher_metadata(signer_address: address): Object<Metadata> acquires Dispatcher {
        let dispatcher = borrow_global<Dispatcher>(@dispatching_check_signature);
        assert!(table::contains(&dispatcher.dispatcher, signer_address), E_DISPATCHER_NOT_FOUND);
        *table::borrow(&dispatcher.dispatcher, signer_address)
    }

    #[view]
    public fun dispatcher_is_exists(signer_address: address): bool acquires Dispatcher {
        let dispatcher = borrow_global<Dispatcher>(@dispatching_check_signature);
        table::contains(&dispatcher.dispatcher, signer_address)
    }

    #[test_only]
    public fun init_module_for_testing(publisher: &signer) {
        init_module(publisher);
    }
}