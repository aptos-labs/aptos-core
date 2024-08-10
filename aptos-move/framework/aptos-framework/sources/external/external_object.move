/// Allowing objects (ObjectCore, together with all resources attached to it) to be stored
/// in external storage, with keeping only the hash onchain. That allows us to retrieve it later._
///
/// Pair of functions `move_existing_object_to_external_storage` and `move_external_object_to_state`
/// allow any deletable object to be moved to external storage, and back to onchain state.
module aptos_framework::external_object {
    use aptos_std::any::{Self, Any};
    use aptos_std::any_map::{Self, AnyMap};
    use aptos_framework::external_unique_state;
    use aptos_framework::event;
    use aptos_framework::object::{Self, ConstructorRef, CreateAtAddressRef, DeleteAndRecreateRef, ObjectCore};

    const EMOVING_TO_STATE_NOT_FINISHED: u64 = 1;
    const EPERMISSION_DOESNT_MATCH: u64 = 2;

    struct ExternalObjectWitness has drop, store { }

    struct ExternalObject has drop, store {
        /// Object address used when object is uncompressed
        object_ref: CreateAtAddressRef,

        resources: AnyMap,

        mut_permission: Any,
    }

    /// Undropable value, which makes sure whole object was consumed,
    /// when moving object from external storage to onchain state.
    struct MovingToStateObject {
        resources: AnyMap,
    }

    #[event]
    struct ObjectMovedToExternalStorage has drop, store {
        object_addr: address,
        hash: u256,
    }

    entry fun initialize_external_object(framework_signer: &signer) {
        external_unique_state::enable_external_storage_for_type<ExternalObject, ExternalObjectWitness>(framework_signer, ExternalObjectWitness {});
    }

    public fun move_existing_object_to_external_storage<P: drop + store>(ref: DeleteAndRecreateRef, resources: AnyMap, mut_permission: P) {
        let object_addr = ref.address_from_delete_and_recreate_ref();
        let object_ref = object::delete_and_can_recreate(ref);

        let compressed_object = ExternalObject {
            object_ref,
            resources,
            mut_permission: any::pack(mut_permission),
        };

        let hash = external_unique_state::move_to_external_storage(compressed_object, &ExternalObjectWitness {});

        event::emit(ObjectMovedToExternalStorage {
            object_addr,
            hash,
        });
    }

    public fun move_external_object_to_state<P: drop + store>(external_bytes: vector<u8>, mut_permission: P): (ConstructorRef, MovingToStateObject) {
        let ExternalObject {
            object_ref,
            resources,
            mut_permission: external_mut_perm,
        } = external_unique_state::move_from_external_storage<ExternalObject, ExternalObjectWitness>(external_bytes, &ExternalObjectWitness {});
        assert!(mut_permission == external_mut_perm.unpack(), EPERMISSION_DOESNT_MATCH);

        let constructor_ref = object::create_object_at_address_from_ref(object_ref);

        (constructor_ref, MovingToStateObject {
            resources: resources,
        })
    }

    public fun get_resources_mut(self: &mut MovingToStateObject): &mut AnyMap {
        &mut self.resources
    }

    public fun destroy_empty(self: MovingToStateObject) {
        assert!(any_map::length(&self.resources) == 0, EMOVING_TO_STATE_NOT_FINISHED);
        let MovingToStateObject {
            resources: _
        } = self;
    }

    // Allow for object API to be called without any permissions, same as non-external objects.

    public entry fun transfer(owner: &signer, external_bytes: vector<u8>, to: address) {
        let ExternalObject {
            object_ref,
            resources,
            mut_permission,
        } = external_unique_state::move_from_external_storage<ExternalObject, ExternalObjectWitness>(external_bytes, &ExternalObjectWitness {});

        let constructor_ref = object::create_object_at_address_from_ref(object_ref);

        object::transfer<ObjectCore>(owner, constructor_ref.object_from_constructor_ref(), to);

        let object_ref = object::delete_and_can_recreate(constructor_ref.generate_delete_and_recreate_ref());
        let compressed_object = ExternalObject {
            object_ref,
            resources,
            mut_permission,
        };

        external_unique_state::move_to_external_storage(compressed_object, &ExternalObjectWitness {});
    }
}
