module aptos_framework::compressed_object {

    use aptos_std::copyable_any_map::{Self, AnyMap};
    use aptos_framework::compressed_state;
    use aptos_framework::object::{Self, ConstructorRef, CreateAtAddressRef, DeleteAndRecreateRef};


    const EDECOMPRESSION_NOT_FINISHED: u64 = 1;

    struct CompressedObjectCore has store, drop, copy {
        owner: address,
        allow_ungated_transfer: bool,
    }

    struct CompressedObject has drop, store {
        /// Object address used when object is uncompressed
        object: CreateAtAddressRef,

        resources: AnyMap,
    }

    // Hot potato, making sure whole object was decompressed.
    struct DecompressingObject {
        resources: AnyMap,
    }

    entry fun initialize_compressed_object(framework_signer: &signer) {
        compressed_state::enable_compression_for_custom_core<CompressedObjectCore>(framework_signer);
    }

    // public fun compress_new_object(object: CreateAtAddressRef, resources: AnyMap, owner: address) {
    //     let compressed_core = CompressedObjectCore {
    //         owner,
    //         allow_ungated_transfer,
    //     };

    //     let compressed_object = CompressedObject {
    //         object,
    //         resources,
    //     };

    //     compressed_state::compress(compressed_core, compressed_object);
    // }

    public fun compress_existing_object(ref: DeleteAndRecreateRef, resources: AnyMap) {
        let (object, owner, allow_ungated_transfer) = object::delete_and_can_recreate(ref);

        let compressed_core = CompressedObjectCore {
            owner,
            allow_ungated_transfer,
        };

        let compressed_object = CompressedObject {
            object,
            resources,
        };

        compressed_state::compress(compressed_core, compressed_object);
    }

    public fun decompress_object(compressed_id: u64, serialized: vector<u8>): (ConstructorRef, DecompressingObject) {
        let (
            CompressedObjectCore {
                owner,
                allow_ungated_transfer: _,
            },
            CompressedObject {
                object,
                resources,
            }
        ) = compressed_state::decompress_and_remove<CompressedObjectCore, CompressedObject>(compressed_id, serialized);

        let constructor_ref = object::create_object_at_address_from_ref(owner, object);

        (constructor_ref, DecompressingObject {
            resources: resources,
        })
    }

    public fun finish_decompressing(object: DecompressingObject) {
        assert!(copyable_any_map::length(&object.resources) == 0, EDECOMPRESSION_NOT_FINISHED);
        let DecompressingObject {
            resources: _
        } = object;
    }
}
