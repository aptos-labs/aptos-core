spec aptos_framework::object {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: It's not possible to create an object twice on the same address.
    /// Criticality: Critical
    /// Implementation: The create_object_internal function includes an assertion to ensure that the object being
    /// created does not already exist at the specified address.
    /// Enforcement: Formally verified via [high-level-req-1](create_object_internal).
    ///
    /// No.: 2
    /// Requirement: Only its owner may transfer an object.
    /// Criticality: Critical
    /// Implementation: The transfer function mandates that the transaction be signed by the owner's address, ensuring
    /// that only the rightful owner may initiate the object transfer.
    /// Enforcement: Audited that it aborts if anyone other than the owner attempts to transfer.
    ///
    /// No.: 3
    /// Requirement: The indirect owner of an object may transfer the object.
    /// Criticality: Medium
    /// Implementation: The owns function evaluates to true when the given address possesses either direct or indirect
    /// ownership of the specified object.
    /// Enforcement: Audited that it aborts if address transferring is not indirect owner.
    ///
    /// No.: 4
    /// Requirement: Objects may never change the address which houses them.
    /// Criticality: Low
    /// Implementation: After creating an object, transfers to another owner may occur. However, the address which
    /// stores the object may not be changed.
    /// Enforcement: This is implied by [high-level-req](high-level requirement 1).
    ///
    /// No.: 5
    /// Requirement: If an ungated transfer is disabled on an object in an indirect ownership chain, a transfer should not
    /// occur.
    /// Criticality: Medium
    /// Implementation: Calling disable_ungated_transfer disables direct transfer, and only TransferRef may trigger
    /// transfers. The transfer_with_ref function is called.
    /// Enforcement: Formally verified via [high-level-req-5](transfer_with_ref).
    ///
    /// No.: 6
    /// Requirement: Object addresses must not overlap with other addresses in different domains.
    /// Criticality: Critical
    /// Implementation: The current addressing scheme with suffixes does not conflict with any existing addresses,
    /// such as resource accounts. The GUID space is explicitly separated to ensure this doesn't happen.
    /// Enforcement: This is true by construction if one correctly ensures the usage of INIT_GUID_CREATION_NUM during
    /// the creation of GUID.
    /// </high-level-req>
    ///
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
        ensures result == Object<T> { inner: object };
    }

    spec create_object(owner_address: address): ConstructorRef {
        pragma aborts_if_is_partial;

        let unique_address = transaction_context::spec_generate_unique_address();
        aborts_if exists<ObjectCore>(unique_address);

        ensures exists<ObjectCore>(unique_address);
        ensures global<ObjectCore>(unique_address) == ObjectCore {
            guid_creation_num: INIT_GUID_CREATION_NUM + 1,
            owner: owner_address,
            allow_ungated_transfer: true,
            transfer_events: event::EventHandle {
                counter: 0,
                guid: guid::GUID {
                    id: guid::ID {
                        creation_num: INIT_GUID_CREATION_NUM,
                        addr: unique_address,
                    }
                }
            }
        };
        ensures result == ConstructorRef { self: unique_address, can_delete: true };
    }

    spec create_sticky_object(owner_address: address): ConstructorRef {
        pragma aborts_if_is_partial;

        let unique_address = transaction_context::spec_generate_unique_address();
        aborts_if exists<ObjectCore>(unique_address);

        ensures exists<ObjectCore>(unique_address);
        ensures global<ObjectCore>(unique_address) == ObjectCore {
            guid_creation_num: INIT_GUID_CREATION_NUM + 1,
            owner: owner_address,
            allow_ungated_transfer: true,
            transfer_events: event::EventHandle {
                counter: 0,
                guid: guid::GUID {
                    id: guid::ID {
                        creation_num: INIT_GUID_CREATION_NUM,
                        addr: unique_address,
                    }
                }
            }
        };
        ensures result == ConstructorRef { self: unique_address, can_delete: false };
    }

    spec create_object_address(source: &address, seed: vector<u8>): address {
        pragma opaque;
        pragma aborts_if_is_strict = false;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_object_address(source, seed);
    }

    spec fun spec_create_user_derived_object_address_impl(source: address, derive_from: address): address;

    spec create_user_derived_object_address_impl(source: address, derive_from: address): address {
        pragma opaque;
        ensures [abstract] result == spec_create_user_derived_object_address_impl(source, derive_from);
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
        ensures result == object.inner;
    }

    spec convert<X: key, Y: key>(object: Object<X>): Object<Y> {
        aborts_if !exists<ObjectCore>(object.inner);
        aborts_if !spec_exists_at<Y>(object.inner);
        ensures result == Object<Y> { inner: object.inner };
    }

    spec create_named_object(creator: &signer, seed: vector<u8>): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let obj_addr = spec_create_object_address(creator_address, seed);
        aborts_if exists<ObjectCore>(obj_addr);

        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
            guid_creation_num: INIT_GUID_CREATION_NUM + 1,
            owner: creator_address,
            allow_ungated_transfer: true,
            transfer_events: event::EventHandle {
                counter: 0,
                guid: guid::GUID {
                    id: guid::ID {
                        creation_num: INIT_GUID_CREATION_NUM,
                        addr: obj_addr,
                    }
                }
            }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: false };
    }

    spec create_user_derived_object(creator_address: address, derive_ref: &DeriveRef): ConstructorRef {
        let obj_addr = spec_create_user_derived_object_address(creator_address, derive_ref.self);
        aborts_if exists<ObjectCore>(obj_addr);

        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
            guid_creation_num: INIT_GUID_CREATION_NUM + 1,
            owner: creator_address,
            allow_ungated_transfer: true,
            transfer_events: event::EventHandle {
                counter: 0,
                guid: guid::GUID {
                    id: guid::ID {
                        creation_num: INIT_GUID_CREATION_NUM,
                        addr: obj_addr,
                    }
                }
            }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: false };
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
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);

        ensures global<account::Account>(addr).guid_creation_num == old(
            global<account::Account>(addr)
        ).guid_creation_num + 1;
        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
            guid_creation_num: INIT_GUID_CREATION_NUM + 1,
            owner: addr,
            allow_ungated_transfer: true,
            transfer_events: event::EventHandle {
                counter: 0,
                guid: guid::GUID {
                    id: guid::ID {
                        creation_num: INIT_GUID_CREATION_NUM,
                        addr: obj_addr,
                    }
                }
            }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: true };
    }

    spec create_object_from_object(creator: &signer): ConstructorRef {
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
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);

        ensures global<ObjectCore>(addr).guid_creation_num == old(global<ObjectCore>(addr)).guid_creation_num + 1;
        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
            guid_creation_num: INIT_GUID_CREATION_NUM + 1,
            owner: addr,
            allow_ungated_transfer: true,
            transfer_events: event::EventHandle {
                counter: 0,
                guid: guid::GUID {
                    id: guid::ID {
                        creation_num: INIT_GUID_CREATION_NUM,
                        addr: obj_addr,
                    }
                }
            }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: true };
    }

    spec create_object_from_guid(creator_address: address, guid: guid::GUID): ConstructorRef {
        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);

        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr) == ObjectCore {
            guid_creation_num: INIT_GUID_CREATION_NUM + 1,
            owner: creator_address,
            allow_ungated_transfer: true,
            transfer_events: event::EventHandle {
                counter: 0,
                guid: guid::GUID {
                    id: guid::ID {
                        creation_num: INIT_GUID_CREATION_NUM,
                        addr: obj_addr,
                    }
                }
            }
        };
        ensures result == ConstructorRef { self: obj_addr, can_delete: true };
    }

    spec create_sticky_object_at_address(owner_address: address, object_address: address): ConstructorRef {
        // TODO(fa_migration)
        pragma verify = false;
    }

    spec create_object_internal(
    creator_address: address,
    object: address,
    can_delete: bool,
    ): ConstructorRef {
        // property 1: Creating an object twice on the same address must never occur.
        /// [high-level-req-1]
        aborts_if exists<ObjectCore>(object);
        ensures exists<ObjectCore>(object);
        // property 6: Object addresses must not overlap with other addresses in different domains.
        ensures global<ObjectCore>(object).guid_creation_num == INIT_GUID_CREATION_NUM + 1;
        ensures result == ConstructorRef { self: object, can_delete };
    }

    spec generate_delete_ref(ref: &ConstructorRef): DeleteRef {
        aborts_if !ref.can_delete;
        ensures result == DeleteRef { self: ref.self };
    }

    spec disable_ungated_transfer(ref: &TransferRef) {
        aborts_if !exists<ObjectCore>(ref.self);
        ensures global<ObjectCore>(ref.self).allow_ungated_transfer == false;
    }

    spec object_from_constructor_ref<T: key>(ref: &ConstructorRef): Object<T> {
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if !spec_exists_at<T>(ref.self);
        ensures result == Object<T> { inner: ref.self };
    }

    spec create_guid(object: &signer): guid::GUID {
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;

        ensures result == guid::GUID {
            id: guid::ID {
                creation_num: object_data.guid_creation_num,
                addr: signer::address_of(object)
            }
        };
    }

    spec new_event_handle<T: drop + store>(
    object: &signer,
    ): event::EventHandle<T> {
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;

        let guid = guid::GUID {
            id: guid::ID {
                creation_num: object_data.guid_creation_num,
                addr: signer::address_of(object)
            }
        };
        ensures result == event::EventHandle<T> {
            counter: 0,
            guid,
        };
    }

    spec object_from_delete_ref<T: key>(ref: &DeleteRef): Object<T> {
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if !spec_exists_at<T>(ref.self);
        ensures result == Object<T> { inner: ref.self };
    }

    spec delete(ref: DeleteRef) {
        aborts_if !exists<ObjectCore>(ref.self);
        ensures !exists<ObjectCore>(ref.self);
    }

    spec set_untransferable(ref: &ConstructorRef) {
        aborts_if !exists<ObjectCore>(ref.self);
        aborts_if exists<Untransferable>(ref.self);
        ensures exists<Untransferable>(ref.self);
        ensures global<ObjectCore>(ref.self).allow_ungated_transfer == false;
    }

    spec enable_ungated_transfer(ref: &TransferRef) {
        aborts_if exists<Untransferable>(ref.self);
        aborts_if !exists<ObjectCore>(ref.self);
        ensures global<ObjectCore>(ref.self).allow_ungated_transfer == true;
    }

    spec generate_transfer_ref(ref: &ConstructorRef): TransferRef {
        aborts_if exists<Untransferable>(ref.self);
        ensures result == TransferRef {
            self: ref.self,
        };
    }

    spec generate_linear_transfer_ref(ref: &TransferRef): LinearTransferRef {
        aborts_if exists<Untransferable>(ref.self);
        aborts_if !exists<ObjectCore>(ref.self);
        let owner = global<ObjectCore>(ref.self).owner;
        ensures result == LinearTransferRef {
            self: ref.self,
            owner,
        };
    }

    spec transfer_with_ref(ref: LinearTransferRef, to: address) {
        aborts_if exists<Untransferable>(ref.self);
        let object = global<ObjectCore>(ref.self);
        aborts_if !exists<ObjectCore>(ref.self);
        /// [high-level-req-5]
        aborts_if object.owner != ref.owner;
        ensures global<ObjectCore>(ref.self).owner == to;
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
    ) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !global<ObjectCore>(object_address).allow_ungated_transfer;
    }

    spec burn<T: key>(_owner: &signer, _object: Object<T>) {
        aborts_if true;
    }

    spec burn_object<T: key>(owner: &signer, object: Object<T>) {
        pragma aborts_if_is_partial;
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if owner(object) != signer::address_of(owner);
        aborts_if is_burnt(object);
    }

    spec unburn<T: key>(original_owner: &signer, object: Object<T>) {
        pragma aborts_if_is_partial;
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !is_burnt(object);
        let tomb_stone = borrow_global<TombStone>(object_address);
        aborts_if tomb_stone.original_owner != signer::address_of(original_owner);
    }

    spec verify_ungated_and_descendant(owner: address, destination: address) {
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        pragma aborts_if_is_partial;
        pragma unroll = MAXIMUM_OBJECT_NESTING;
        aborts_if !exists<ObjectCore>(destination);
        aborts_if !global<ObjectCore>(destination).allow_ungated_transfer;
        // aborts_if exists i in 0..g_roll:
        //     owner != global<ObjectCore>(destination).owner && !exists<ObjectCore>(get_transfer_address(destination, i));
        // aborts_if exists i in 0..g_roll:
        //     owner != global<ObjectCore>(destination).owner && !global<ObjectCore>(get_transfer_address(destination, i)).allow_ungated_transfer;
        // property 3: The 'indirect' owner of an object may transfer the object.
        // ensures exists i in 0..MAXIMUM_OBJECT_NESTING:
        //     owner == get_transfer_address(destination, i);
    }

    // Helper function for property 3
    // spec fun get_transfer_address(addr: address, roll: u64): address {
    //     let i = roll;
    //     if ( i > 0 )
    //     { get_transfer_address(global<ObjectCore>(addr).owner, i - 1) }
    //     else
    //     { global<ObjectCore>(addr).owner }
    // }

    spec ungated_transfer_allowed<T: key>(object: Object<T>): bool {
        aborts_if !exists<ObjectCore>(object.inner);
        ensures result == global<ObjectCore>(object.inner).allow_ungated_transfer;
    }

    spec is_owner<T: key>(object: Object<T>, owner: address): bool {
        aborts_if !exists<ObjectCore>(object.inner);
        ensures result == (global<ObjectCore>(object.inner).owner == owner);
    }

    spec owner<T: key>(object: Object<T>): address {
        aborts_if !exists<ObjectCore>(object.inner);
        ensures result == global<ObjectCore>(object.inner).owner;
    }

    spec owns<T: key>(object: Object<T>, owner: address): bool {
        pragma aborts_if_is_partial;
        let current_address_0 = object.inner;
        let object_0 = global<ObjectCore>(current_address_0);
        let current_address = object_0.owner;
        aborts_if object.inner != owner && !exists<ObjectCore>(object.inner);
        ensures current_address_0 == owner ==> result == true;
    }

    spec root_owner<T: key>(object: Object<T>): address {
        pragma aborts_if_is_partial;
    }

    // Helper function
    spec fun spec_create_object_address(source: address, seed: vector<u8>): address;

    spec fun spec_create_user_derived_object_address(source: address, derive_from: address): address;

    spec fun spec_create_guid_object_address(source: address, creation_num: u64): address;
}
