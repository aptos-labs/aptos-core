module dispatching_check_signature::check_signature {
    use std::option::{Self, Option};
    use std::string;
    use std::vector;    
    use std::signer;
    use aptos_std::bcs;

    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::function_info::{Self, FunctionInfo};
    use aptos_framework::fungible_asset;
    use aptos_framework::object;

    use dispatching_check_signature::storage;

    const VERIFY_SUCCESS: u128 = 0;
    const VERIFY_FAILURE: u128 = 1;

    /// Check the signature of the given module address.
    public fun check_signature(
      module_address: address,
      digest_hash: vector<u8>,
      signature_bytes: vector<u8>
    ): bool {
      if (storage::dispatcher_is_exists(module_address)) {
          let data = vector::empty<u8>();
          vector::append(&mut data, digest_hash);
          vector::append(&mut data, signature_bytes);
          
          storage::insert(module_address, data);
          
          let result = dispatch(module_address);
          if (option::is_some(&result)) {
            return *option::borrow(&result) == VERIFY_SUCCESS
          };
          return false
      };

      true
    }

    /// Register the dispatchable function of the dispatcher.
    public fun register_dispatchable(signer: &signer) {
        let cb = function_info::new_function_info(
            signer,
            string::utf8(b"check_signature_example"),
            string::utf8(b"verify"),
        );

        let signer_address = signer::address_of(signer);

        register(cb, signer_address)
    }

    fun register(callback: FunctionInfo, signer_address: address) {
        let constructor_ref = object::create_named_object(&storage::get_signer(), bcs::to_bytes(&signer_address));
        let metadata = fungible_asset::add_fungibility(
            &constructor_ref,
            option::none(),
            string::utf8(b"check_signature"),
            string::utf8(b"dispatch"),
            0,
            string::utf8(b""),
            string::utf8(b""),
        );
        dispatchable_fungible_asset::register_derive_supply_dispatch_function(
            &constructor_ref,
            option::some(callback),
        );

        storage::set_dispatcher_metadata(signer_address, metadata);
    }

    fun dispatch(signer_address: address): Option<u128> {
      let metadata = storage::dispatcher_metadata(signer_address);
      dispatchable_fungible_asset::derived_supply(metadata)
    }
}