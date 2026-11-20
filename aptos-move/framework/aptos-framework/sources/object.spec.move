spec aptos_framework::object {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: It's not possible to create an object twice on the same address.
    /// Criticality: Critical
    /// Implementation: The create_object_internal function includes an assertion to ensure that the object being
    /// created does not already exist at the specified address.
    /// Enforcement: Formally verified via [high-level-req-1.1](create_object_internal) and
    /// [high-level-req-1.2](create_sticky_object_at_address).
    ///
    /// No.: 2
    /// Requirement: Only its owner may transfer an object.
    /// Criticality: Critical
    /// Implementation: The transfer function mandates that the transaction be signed by the owner's address, ensuring
    /// that only the rightful owner may initiate the object transfer.
    /// Enforcement: Formally verified via [high-level-req-2](transfer_raw) that it aborts unless the signer is reached by
    /// the owner chain of the object within the nesting limit.
    ///
    /// No.: 3
    /// Requirement: The indirect owner of an object may transfer the object.
    /// Criticality: Medium
    /// Implementation: The owns function evaluates to true when the given address possesses either direct or indirect
    /// ownership of the specified object.
    /// Enforcement: Formally verified via [high-level-req-3](transfer_raw) that the abort set is exact: an indirect owner
    /// whose chain allows ungated transfer at every object can transfer; `owns` is characterised exactly as well.
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
        pragma verify = true;
    }

    spec fun spec_exists_at<T: key>(object: address): bool;

    spec exists_at<T: key>(object: address): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_exists_at<T>(object);
    }


    spec address_to_object<T: key>(object: address): Object<T> {
        pragma opaque;
        aborts_if !exists<ObjectCore>(object);
        aborts_if !spec_exists_at<T>(object);
        ensures result == Object<T> { inner: object };
    }

    spec create_object(owner_address: address): ConstructorRef {
        pragma opaque;

        let unique_address = transaction_context::spec_generate_unique_address();
        aborts_if exists<ObjectCore>(unique_address);
        modifies global<ObjectCore>(unique_address);

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
        pragma opaque;

        let unique_address = transaction_context::spec_generate_unique_address();
        aborts_if exists<ObjectCore>(unique_address);
        modifies global<ObjectCore>(unique_address);

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
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_object_address(source, seed);
    }

    spec fun spec_create_user_derived_object_address_impl(source: address, derive_from: address): address;

    spec create_user_derived_object_address_impl(source: address, derive_from: address): address {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_user_derived_object_address_impl(source, derive_from);
    }

    spec create_user_derived_object_address(source: address, derive_from: address): address {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_user_derived_object_address(source, derive_from);
    }

    spec create_guid_object_address(source: address, creation_num: u64): address {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_guid_object_address(source, creation_num);
    }

    spec object_address<T: key>(self: &Object<T>): address {
        aborts_if false;
        ensures result == self.inner;
    }

    spec convert<X: key, Y: key>(self: Object<X>): Object<Y> {
        aborts_if !exists<ObjectCore>(self.inner);
        aborts_if !spec_exists_at<Y>(self.inner);
        ensures result == Object<Y> { inner: self.inner };
    }

    spec create_named_object(creator: &signer, seed: vector<u8>): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let obj_addr = spec_create_object_address(creator_address, seed);
        modifies global<ObjectCore>(obj_addr);
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
        use std::features;
        use std::features::DEFAULT_ACCOUNT_RESOURCE;
        pragma opaque;
        let addr = signer::address_of(creator);
        let account_exists_pre = exists<account::Account>(addr);
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);

        aborts_if !account_exists_pre && !feature_on;
        aborts_if !account_exists_pre && feature_on
            && (addr == @vm_reserved || addr == @aptos_framework || addr == @aptos_token);
        aborts_if !account_exists_pre && feature_on
            && len(bcs::to_bytes(addr)) != 32;

        let creation_num = if (account_exists_pre) {
            global<account::Account>(addr).guid_creation_num
        } else {
            2
        };
        aborts_if creation_num + 1 >= account::MAX_GUID_CREATION_NUM;

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
        modifies global<account::Account>(addr);
        modifies global<ObjectCore>(obj_addr);

        ensures exists<account::Account>(addr);
        ensures global<account::Account>(addr).guid_creation_num == creation_num + 1;
        // Only the GUID counter of an existing account changes.
        ensures account_exists_pre ==> global<account::Account>(addr)
            == update_field(old(global<account::Account>(addr)), guid_creation_num, creation_num + 1);
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
        pragma opaque;
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
        modifies global<ObjectCore>(addr);
        modifies global<ObjectCore>(obj_addr);

        ensures global<ObjectCore>(addr) == update_field(object_data, guid_creation_num, creation_num + 1);
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
        pragma opaque;
        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);
        modifies global<ObjectCore>(obj_addr);

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
        /// [high-level-req-1.2]
        modifies global<ObjectCore>(object_address);
        aborts_if exists<ObjectCore>(object_address);
        ensures exists<ObjectCore>(object_address);
        ensures global<ObjectCore>(object_address).guid_creation_num == INIT_GUID_CREATION_NUM + 1;
        ensures result == ConstructorRef { self: object_address, can_delete: false };
    }

    spec create_object_internal(
        creator_address: address,
        object: address,
        can_delete: bool,
    ): ConstructorRef {
        // property 1: Creating an object twice on the same address must never occur.
        pragma opaque;
        /// [high-level-req-1.1]
        aborts_if exists<ObjectCore>(object);
        ensures exists<ObjectCore>(object);
        // property 6: Object addresses must not overlap with other addresses in different domains.
        modifies global<ObjectCore>(object);
        ensures global<ObjectCore>(object) == ObjectCore {
            guid_creation_num: INIT_GUID_CREATION_NUM + 1,
            owner: creator_address,
            allow_ungated_transfer: true,
            transfer_events: event::EventHandle {
                counter: 0,
                guid: guid::GUID {
                    id: guid::ID {
                        creation_num: INIT_GUID_CREATION_NUM,
                        addr: object,
                    }
                }
            }
        };
        ensures result == ConstructorRef { self: object, can_delete };
    }

    spec generate_delete_ref(self: &ConstructorRef): DeleteRef {
        aborts_if !self.can_delete;
        ensures result == DeleteRef { self: self.self };
    }

    spec disable_ungated_transfer(self: &TransferRef) {
        aborts_if !exists<ObjectCore>(self.self);
        ensures global<ObjectCore>(self.self).allow_ungated_transfer == false;
    }

    spec object_from_constructor_ref<T: key>(self: &ConstructorRef): Object<T> {
        aborts_if !exists<ObjectCore>(self.self);
        aborts_if !spec_exists_at<T>(self.self);
        ensures result == Object<T> { inner: self.self };
    }

    spec create_guid(object: &signer): guid::GUID {
        pragma opaque;
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;
        modifies global<ObjectCore>(signer::address_of(object));
        ensures global<ObjectCore>(signer::address_of(object)) == update_field(
            object_data, guid_creation_num, object_data.guid_creation_num + 1);

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
        pragma opaque;
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;
        modifies global<ObjectCore>(signer::address_of(object));
        ensures global<ObjectCore>(signer::address_of(object)) == update_field(
            object_data, guid_creation_num, object_data.guid_creation_num + 1);

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

    spec object_from_delete_ref<T: key>(self: &DeleteRef): Object<T> {
        aborts_if !exists<ObjectCore>(self.self);
        aborts_if !spec_exists_at<T>(self.self);
        ensures result == Object<T> { inner: self.self };
    }

    spec delete(self: DeleteRef) {
        aborts_if !exists<ObjectCore>(self.self);
        ensures !exists<ObjectCore>(self.self);
    }

    spec generate_signer_for_extending {
        pragma opaque;
        aborts_if false;
        ensures result == spec_generate_signer_for_extending(self);
        // Not implied by the clause above: `spec_create_signer` is uninterpreted, so
        // under `opaque` a caller cannot otherwise name the signer's address.
        ensures signer::address_of(result) == self.self;
    }

    spec set_untransferable(self: &ConstructorRef) {
        aborts_if !exists<ObjectCore>(self.self);
        aborts_if exists<Untransferable>(self.self);
        ensures exists<Untransferable>(self.self);
        ensures global<ObjectCore>(self.self).allow_ungated_transfer == false;
    }

    spec enable_ungated_transfer(self: &TransferRef) {
        aborts_if exists<Untransferable>(self.self);
        aborts_if !exists<ObjectCore>(self.self);
        ensures global<ObjectCore>(self.self).allow_ungated_transfer == true;
    }

    spec generate_transfer_ref(self: &ConstructorRef): TransferRef {
        aborts_if exists<Untransferable>(self.self);
        ensures result == TransferRef {
            self: self.self,
        };
    }

    spec generate_linear_transfer_ref(self: &TransferRef): LinearTransferRef {
        pragma opaque;
        aborts_if exists<Untransferable>(self.self);
        aborts_if !exists<ObjectCore>(self.self);
        let owner = global<ObjectCore>(self.self).owner;
        ensures result == LinearTransferRef {
            self: self.self,
            owner,
        };
    }

    spec transfer_with_ref(self: LinearTransferRef, to: address) {
        aborts_if exists<Untransferable>(self.self);
        let object = global<ObjectCore>(self.self);
        aborts_if !exists<ObjectCore>(self.self);
        /// [high-level-req-5]
        aborts_if object.owner != self.owner;
        ensures global<ObjectCore>(self.self).owner == to;
    }

    spec transfer_call(
        owner: &signer,
        object: address,
        to: address,
    ) {
        include TransferRawAbortsIf { owner_address: signer::address_of(owner) };
        ensures global<ObjectCore>(object).owner == to;
    }

    spec transfer<T: key>(
        owner: &signer,
        object: Object<T>,
        to: address,
    ) {
        include TransferRawAbortsIf { owner_address: signer::address_of(owner), object: object.inner };
        ensures global<ObjectCore>(object.inner).owner == to;
    }

    spec transfer_raw(
        owner: &signer,
        object: address,
        to: address,
    ) {
        let owner_address = signer::address_of(owner);
        aborts_if !exists<ObjectCore>(object);
        aborts_if !global<ObjectCore>(object).allow_ungated_transfer;
        let first_owner = global<ObjectCore>(object).owner;
        /// [high-level-req-2]
        aborts_if !spec_owns_at(owner_address, first_owner, 0);
        /// [high-level-req-3]
        aborts_if !spec_ungated_descendant_at(owner_address, first_owner, 0);
        ensures global<ObjectCore>(object).owner == to;
    }

    spec transfer_to_object<O: key, T: key> (
        owner: &signer,
        object: Object<O>,
        to: Object<T>,
    ) {
        include TransferRawAbortsIf { owner_address: signer::address_of(owner), object: object.inner };
        ensures global<ObjectCore>(object.inner).owner == to.inner;
    }

    /// The signer is reached by the owner walk from `object`, and `object` and every object passed on
    /// the way allow ungated transfer.
    spec schema TransferRawAbortsIf {
        owner_address: address;
        object: address;
        aborts_if !spec_ungated_descendant(owner_address, object);
    }

    spec burn<T: key>(owner: &signer, object: Object<T>) {
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if owner(object) != signer::address_of(owner);
        aborts_if exists<TombStone>(object_address);
        ensures exists<TombStone>(object_address);
        ensures is_owner(object, signer::address_of(owner));
    }

    spec burn_object_with_transfer<T: key>(owner: &signer, object: Object<T>) {
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if owner(object) != signer::address_of(owner);
        aborts_if is_burnt(object);
    }

    spec unburn<T: key>(original_owner: &signer, object: Object<T>) {
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !is_burnt(object);
        let tomb_stone = borrow_global<TombStone>(object_address);
        let original_owner_address = signer::address_of(original_owner);
        let object_current_owner = borrow_global<ObjectCore>(object_address).owner;
        // Only the current owner, or the original owner of an object held by the burn address, can unburn.
        aborts_if object_current_owner != original_owner_address && object_current_owner != BURN_ADDRESS;
        aborts_if object_current_owner != original_owner_address && tomb_stone.original_owner != original_owner_address;
        ensures !exists<TombStone>(object_address);
    }

    spec verify_ungated_and_descendant(owner: address, destination: address) {
        pragma opaque;
        aborts_if !spec_ungated_descendant(owner, destination);
    }

    /// `destination` exists, allows ungated transfer, and its owner chain reaches `owner` within the
    /// nesting limit with every intermediate object allowing ungated transfer.
    spec fun spec_ungated_descendant(owner: address, destination: address): bool {
        exists<ObjectCore>(destination) && global<ObjectCore>(destination).allow_ungated_transfer
            && spec_ungated_descendant_at(owner, global<ObjectCore>(destination).owner, 0)
    }

    /// The walk from `current`, after `count` hops, reaches `owner` within the nesting limit through
    /// objects that allow ungated transfer.
    spec fun spec_ungated_descendant_at(owner: address, current: address, count: num): bool {
        if (current == owner) {
            true
        } else if (count + 1 >= MAXIMUM_OBJECT_NESTING) {
            false
        } else {
            exists<ObjectCore>(current) && global<ObjectCore>(current).allow_ungated_transfer
                && spec_ungated_descendant_at(owner, global<ObjectCore>(current).owner, count + 1)
        }
    }

    /// The owner walk from `current`, after `count` hops, reaches `owner` within the nesting limit.
    spec fun spec_owns_at(owner: address, current: address, count: num): bool {
        if (current == owner) {
            true
        } else if (count + 1 >= MAXIMUM_OBJECT_NESTING) {
            false
        } else if (!exists<ObjectCore>(current)) {
            false
        } else {
            spec_owns_at(owner, global<ObjectCore>(current).owner, count + 1)
        }
    }

    /// The owner walk from `current`, after `count` hops, runs into the nesting limit.
    spec fun spec_owns_limit_at(owner: address, current: address, count: num): bool {
        if (current == owner) {
            false
        } else if (count + 1 >= MAXIMUM_OBJECT_NESTING) {
            true
        } else if (!exists<ObjectCore>(current)) {
            false
        } else {
            spec_owns_limit_at(owner, global<ObjectCore>(current).owner, count + 1)
        }
    }

    spec ungated_transfer_allowed<T: key>(self: Object<T>): bool {
        pragma opaque;
        aborts_if !exists<ObjectCore>(self.inner);
        ensures result == global<ObjectCore>(self.inner).allow_ungated_transfer;
    }

    spec is_owner<T: key>(object: Object<T>, owner: address): bool {
        aborts_if !exists<ObjectCore>(object.inner);
        ensures result == (global<ObjectCore>(object.inner).owner == owner);
    }

    spec owner<T: key>(self: Object<T>): address {
        pragma opaque;
        aborts_if !exists<ObjectCore>(self.inner);
        ensures result == global<ObjectCore>(self.inner).owner;
    }

    /// The end of the ownership chain is not an object.
    spec root_owner<T: key>(self: Object<T>): address {
        aborts_if !exists<ObjectCore>(self.inner);
        ensures !exists<ObjectCore>(result);
    }

    spec owns<T: key>(object: Object<T>, owner: address): bool {
        pragma opaque;
        aborts_if spec_owns_aborts(object.inner, owner);
        ensures result == spec_owns(object.inner, owner);
    }

    /// `owns` aborts: the object does not exist, or the walk runs into the nesting limit.
    spec fun spec_owns_aborts(object: address, owner: address): bool {
        !exists<ObjectCore>(object)
            || object != owner && spec_owns_limit_at(owner, global<ObjectCore>(object).owner, 0)
    }

    /// `owner` owns `object` directly or indirectly within the nesting limit.
    spec fun spec_owns(object: address, owner: address): bool {
        object == owner || spec_owns_at(owner, global<ObjectCore>(object).owner, 0)
    }

    // Helper function
    spec fun spec_create_object_address(source: address, seed: vector<u8>): address;

    spec fun spec_create_user_derived_object_address(source: address, derive_from: address): address;

    spec fun spec_create_guid_object_address(source: address, creation_num: u64): address;

    spec fun spec_generate_signer_for_extending(ref: &ExtendRef): signer {
        aptos_framework::create_signer::spec_create_signer(ref.self)
    }
}
