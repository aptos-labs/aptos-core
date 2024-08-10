
module aptos_framework::compressed_state {
    use std::bcs;
    use std::error;
    use std::event;
    use std::hash;
    use std::signer;
    use std::string::String;
    use aptos_std::from_bcs;
    use aptos_std::type_info;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_framework::util;
    use aptos_framework::unique_key;

    friend aptos_framework::compressed_object;

    const EHASH_DOESNT_MATCH: u64 = 1;
    /// compressed_id already exists
    const ECOMPRESSED_ID_ALREADY_PRESENT: u64 = 2;

    const ECORE_COMPRESSION_ALREADY_REGISTERED: u64 = 3;

    const ETYPE_MISMATCH: u64 = 4;


    struct CompressedCore<T: store + drop + copy> has store, drop, copy {
        hash: address, // TODO change to AggregatorSnapshot
        on_chain_core: T,
    }

    /// Not using Any, to not require unnecessary double serialization.
    struct TypedValue<V: drop + store> has drop, store {
        type_name: String,
        value: V
    }

    // hot potato
    struct MutableHandle<T: store + drop + copy, V: drop + store> {
        compressed_id: u64,
        on_chain_core: T,
        typed_value: V,
    }

    struct CompressedState<T: store + drop + copy> has key {
        table: SmartTable<u64, CompressedCore<T>>,
    }

    #[event]
    struct Compress<T: store + drop + copy, V: drop + store> has drop, store {
        compressed_id: u64,
        core: CompressedCore<T>,
        typed_value: TypedValue<V>,
    }

    public(friend) fun enable_compression_for_custom_core<T: store + drop + copy>(framework_signer: &signer) {
        let compressed_state = CompressedState<T> {
            table: smart_table::new(),
        };
        assert!(!exists<CompressedState<T>>(signer::address_of(framework_signer)), error::invalid_argument(ECORE_COMPRESSION_ALREADY_REGISTERED));
        move_to(framework_signer, compressed_state);
    }

    public fun compress<T: store + drop + copy, V: drop + store>(on_chain_core: T, value: V): u64 acquires CompressedState {
        let compressed_id = unique_key::generate_unique_key(@aptos_framework);
        compress_impl(compressed_id, on_chain_core, value)
    }

    fun compress_impl<T: store + drop + copy, V: drop + store>(compressed_id: u64, on_chain_core: T, value: V): u64 acquires CompressedState {
        let typed_value = TypedValue<V> {
            type_name: type_info::type_name<T>(),
            value,
        };
        let hash = hash(&typed_value);

        let core = CompressedCore {
            hash,
            on_chain_core,
        };
        let event = Compress<T, V> {
            compressed_id,
            core: copy core,
            typed_value,
        };
        event::emit(event);

        let compressed_state = borrow_global_mut<CompressedState<T>>(@aptos_framework);

        assert!(!smart_table::contains(&compressed_state.table, compressed_id), error::invalid_state(ECOMPRESSED_ID_ALREADY_PRESENT));

        smart_table::add(&mut compressed_state.table, compressed_id, core);
        compressed_id
    }

    public fun get_hash<T: store + drop + copy>(compressed_id: u64): address acquires CompressedState {
        let compressed_state = borrow_global<CompressedState<T>>(@aptos_framework);

        smart_table::borrow(&compressed_state.table, compressed_id).hash
    }

    public fun get_onchain_data<T: store + drop + copy>(compressed_id: u64): T acquires CompressedState {
        let compressed_state = borrow_global<CompressedState<T>>(@aptos_framework);

        smart_table::borrow(&compressed_state.table, compressed_id).on_chain_core
    }

    public fun get<T: store + drop + copy, V: drop + store + copy>(compressed_id: u64, serialized: vector<u8>): (T, V) acquires CompressedState {
        let compressed_state = borrow_global<CompressedState<T>>(@aptos_framework);
        let core = smart_table::borrow(&compressed_state.table, compressed_id);

        let value = deserialize_value<V>(core.hash, serialized);
        (core.on_chain_core, value)
    }

    // what permissions are needed here? can we restrict this being called only from modules that define T and V (just low borrow)
    public fun decompress_and_remove<T: store + drop + copy, V: drop + store>(compressed_id: u64, serialized: vector<u8>): (T, V) acquires CompressedState {
        let compressed_state = borrow_global_mut<CompressedState<T>>(@aptos_framework);
        let core = smart_table::remove(&mut compressed_state.table, compressed_id);

        let value = deserialize_value<V>(core.hash, serialized);
        (core.on_chain_core, value)
    }

    public fun borrow_mut<T: store + drop + copy, V: drop + store>(compressed_id: u64, serialized: vector<u8>): MutableHandle<T, V> acquires CompressedState {
        let (on_chain_core, value) = decompress_and_remove<T, V>(compressed_id, serialized);

        MutableHandle {
            compressed_id,
            on_chain_core,
            value,
        }
    }

    public fun handle_get_mut_on_chain_core<T: store + drop + copy, V: drop + store>(handle: &mut MutableHandle<T, V>): &mut T {
        &mut handle.on_chain_core
    }

    public fun handle_get_mut_value<T: store + drop + copy, V: drop + store>(handle: &mut MutableHandle<T, V>): &mut V {
        &mut handle.value
    }

    public fun handle_store<T: store + drop + copy, V: drop + store>(handle: MutableHandle<T, V>) {
        let MutableHandle {
            compressed_id,
            on_chain_core,
            value,
        } = handle;
        compress_impl(compressed_id, on_chain_core, value)
    }


    public(friend) fun deserialize_value<V: drop + store>(hash: address, serialized: vector<u8>): V {
        let data_hash = from_bcs::to_address(hash::sha3_256(serialized));

        assert!(data_hash == hash, error::invalid_argument(EHASH_DOESNT_MATCH));
        let TypedValue {
            value,
            type_name,
        } = util::from_bytes<TypedValue<V>>(serialized);
        // TODO is deserialization from wrong type, and then checking for correct type and aborting safe?
        assert!(type_info::type_name<V>() == type_name, error::invalid_argument(ETYPE_MISMATCH));
        value
    }

    public(friend) inline fun hash_bytes(data: vector<u8>): address {
        from_bcs::to_address(hash::sha3_256(data))
    }

    public(friend) inline fun hash_struct<T>(value: &T): address {
        // TODO create delayed hash -> address, to support aggregators.
        hash_bytes(bcs::to_bytes(value))
    }
}
