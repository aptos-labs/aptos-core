spec aptos_framework::object {

    spec module {
        pragma aborts_if_is_strict;
    }

    spec fun spec_exists_at<T: key>(object: address): bool;

    spec exists_at<T: key>(object: address): bool {
        pragma opaque;
        ensures [abstract] result == spec_exists_at<T>(object);
    }

    spec address_to_object<T: key>(object: address): Object<T> {
        aborts_if !exists<ObjectCore>(object);
        aborts_if !spec_exists_at<T>(object);
    }

    spec create_object(owner_address: address): ConstructorRef{
        use std::features;
        pragma aborts_if_is_partial;

        // TODO: native function generate_unique_address() cause an abort
        aborts_if !features::spec_is_enabled(features::APTOS_UNIQUE_IDENTIFIERS);
        aborts_if exists<ObjectCore>(transaction_context::spec_generate_unique_address());
    }

    spec create_sticky_object(owner_address: address): ConstructorRef{
        use std::features;
        pragma aborts_if_is_partial;

        // TODO: native function generate_unique_address() cause an abort
        aborts_if !features::spec_is_enabled(features::APTOS_UNIQUE_IDENTIFIERS);
        aborts_if exists<ObjectCore>(transaction_context::spec_generate_unique_address());
    }

    spec create_object_address(source: &address, seed: vector<u8>): address {
        pragma opaque;
        pragma aborts_if_is_strict = false;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_object_address(source, seed);
    }

    spec create_user_derived_object_address(source: address, derive_from: address): address {
        pragma opaque;
        pragma aborts_if_is_strict = false;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_user_derived_object_address(source, derive_from);
    }

    spec create_guid_object_address(source: address, creation_num: u64): address {
        pragma opaque;
        pragma aborts_if_is_strict = false;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_guid_object_address(source, creation_num);
    }

    spec object_address<T: key>(object: &Object<T>): address {
        aborts_if false;
    }

    spec convert<X: key, Y: key>(object: Object<X>): Object<Y> {
        aborts_if !exists<ObjectCore>(object.inner);
        aborts_if !spec_exists_at<Y>(object.inner);
    }

    spec create_named_object(creator: &signer, seed: vector<u8>): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let obj_addr = spec_create_object_address(creator_address, seed);
        aborts_if exists<ObjectCore>(obj_addr);
    }

    spec create_user_derived_object(creator_address: address, derive_ref: &DeriveRef): ConstructorRef {
        let obj_addr = spec_create_user_derived_object_address(creator_address, derive_ref.self);
        aborts_if exists<ObjectCore>(obj_addr);
    }

    spec create_object_from_account(creator: &signer): ConstructorRef {
        aborts_if !exists<account::Account>(signer::address_of(creator));
        //Guid properties
        let object_data = global<account::Account>(signer::address_of(creator));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;
        aborts_if object_data.guid_creation_num + 1 >= account::MAX_GUID_CREATION_NUM;
        let creation_num = object_data.guid_creation_num;
        let addr = signer::address_of(creator);

        let guid = guid::GUID {
            id: guid::ID {
                creation_num,
                addr,
            }
        };

        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec,vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);
    }

    spec create_object_from_object(creator: &signer): ConstructorRef{
        aborts_if !exists<ObjectCore>(signer::address_of(creator));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(creator));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;
        let creation_num = object_data.guid_creation_num;
        let addr = signer::address_of(creator);

        let guid = guid::GUID {
            id: guid::ID {
                creation_num,
                addr,
            }
        };

        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec,vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);
    }

    spec create_object_from_guid(creator_address: address, guid: guid::GUID): ConstructorRef {
        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec,vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);
    }

    spec create_object_internal(
        creator_address: address,
        object: address,
        can_delete: bool,
    ): ConstructorRef {
        aborts_if exists<ObjectCore>(object);
    }

    spec generate_delete_ref(ref: &ConstructorRef): DeleteRef {
        aborts_if !ref.can_delete;
    }

    spec disable_ungated_transfer(ref: &TransferRef) {
        aborts_if !exists<ObjectCore>(ref.self);
    }

    spec object_from_constructor_ref<T: key>(ref: &ConstructorRef): Object<T> {
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if !spec_exists_at<T>(ref.self);
    }

    spec create_guid(object: &signer): guid::GUID{
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;
    }

    spec new_event_handle<T: drop + store>(
        object: &signer,
    ): event::EventHandle<T>{
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;
    }

    spec object_from_delete_ref<T: key>(ref: &DeleteRef): Object<T> {
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if !spec_exists_at<T>(ref.self);
    }

    spec delete(ref: DeleteRef) {
        aborts_if !exists<ObjectCore>(ref.self);
    }

    spec enable_ungated_transfer(ref: &TransferRef) {
        aborts_if !exists<ObjectCore>(ref.self);
    }

    spec generate_linear_transfer_ref(ref: &TransferRef):LinearTransferRef {
        aborts_if !exists<ObjectCore>(ref.self);
    }

    spec transfer_with_ref(ref: LinearTransferRef, to: address){
        let object = global<ObjectCore>(ref.self);
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if object.owner != ref.owner;
    }

    spec transfer_call(
        owner: &signer,
        object: address,
        to: address,
    ) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        aborts_if !exists<ObjectCore>(object);
        aborts_if !global<ObjectCore>(object).allow_ungated_transfer;
    }

    spec transfer<T: key>(
        owner: &signer,
        object: Object<T>,
        to: address,
    ) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !global<ObjectCore>(object_address).allow_ungated_transfer;
    }

    spec transfer_raw(
        owner: &signer,
        object: address,
        to: address,
    ) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        aborts_if !exists<ObjectCore>(object);
        aborts_if !global<ObjectCore>(object).allow_ungated_transfer;
    }

    spec transfer_to_object<O: key, T: key> (
        owner: &signer,
        object: Object<O>,
        to: Object<T>,
    ){
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !global<ObjectCore>(object_address).allow_ungated_transfer;
    }

    spec verify_ungated_and_descendant(owner: address, destination: address) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        aborts_if !exists<ObjectCore>(destination);
        aborts_if !global<ObjectCore>(destination).allow_ungated_transfer;
    }

    spec ungated_transfer_allowed<T: key>(object: Object<T>): bool {
        aborts_if !exists<ObjectCore>(object.inner);
    }

    spec is_owner<T: key>(object: Object<T>, owner: address): bool{
        aborts_if !exists<ObjectCore>(object.inner);
    }

    spec owner<T: key>(object: Object<T>): address{
        aborts_if !exists<ObjectCore>(object.inner);
    }

    spec owns<T: key>(object: Object<T>, owner: address): bool {
        aborts_if object.inner != owner && !exists<ObjectCore>(object.inner);
    }

    // Helper function
    spec fun spec_create_object_address(source: address, seed: vector<u8>): address;

    spec fun spec_create_user_derived_object_address(source: address, derive_from: address): address;

    spec fun spec_create_guid_object_address(source: address, creation_num: u64): address;

}
