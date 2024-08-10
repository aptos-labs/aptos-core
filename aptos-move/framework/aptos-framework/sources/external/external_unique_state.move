/// Onchain resource containing a set of ExternalValues.
/// Set is without duplicates - i.e. same value cannot be stored twice.
///
/// Replaces `move_to` / `borrow_global_mut` / `move_from`
/// with `move_to_external_storage` / `borrow_mut` / `move_from_external_storage`
///
/// Provides equivalent access restrictions as `move_to` / `borrow_global_mut` / `move_from`
/// byte instructions have (which are that only declaring module can call them),
/// via each type having a corresponding witness that needs to be provided.
/// (if the module doesn't give out the witness)
module aptos_framework::external_unique_state {
    use std::error;
    use std::signer;
    use aptos_framework::verified_external_value::{Self, ExternalValuesSet};

    friend aptos_framework::external_object;

    const EHASH_DOESNT_MATCH: u64 = 1;
    /// compressed_id already exists
    const ECOMPRESSED_ID_ALREADY_PRESENT: u64 = 2;
    const ECORE_COMPRESSION_ALREADY_REGISTERED: u64 = 3;
    const ETYPE_MISMATCH: u64 = 4;
    const EWITNESS_MISMATCH: u64 = 4;

    /// Resource containing all ExternalValues of a given type.
    /// It also keeps a witness, which is required to be provided in order to access values of this type.
    struct ExternalUniqueState<T: drop + store, V: store> has key {
        witness: V,
        values: ExternalValuesSet<T>,
    }

    /// A handle containing the value, which can be mutated, and then stored back.
    struct MutableHandle<T: drop + store> {
        typed_value: T,
    }

    /// Registers a particular type to be able to be stored in external state, with access guarded by the provided witness.
    public(friend) fun enable_external_storage_for_type<T: drop + store, V: store>(framework_signer: &signer, witness: V) {
        let compressed_state = ExternalUniqueState<T, V> {
            witness: witness,
            values: verified_external_value::new_set()
        };
        assert!(!exists<ExternalUniqueState<T, V>>(signer::address_of(framework_signer)), error::invalid_argument(ECORE_COMPRESSION_ALREADY_REGISTERED));
        move_to(framework_signer, compressed_state);
    }

    public fun move_to_external_storage<T: drop + store, V: store>(value: T, witness: &V): u256 acquires ExternalUniqueState {
        let external_state = borrow_global_mut<ExternalUniqueState<T, V>>(@aptos_framework);
        assert!(witness == &external_state.witness, error::invalid_argument(EWITNESS_MISMATCH));

        let external_value = verified_external_value::move_to_external_storage(value);
        let hash = external_value.get_hash();
        external_state.values.add(external_value);
        hash
    }

    public fun get_copy<T: store + drop + copy, V: store>(external_bytes: vector<u8>, witness: &V): T acquires ExternalUniqueState {
        let external_state = borrow_global<ExternalUniqueState<T, V>>(@aptos_framework);
        assert!(witness == &external_state.witness, error::invalid_argument(EWITNESS_MISMATCH));
        external_state.values.get_copy(verified_external_value::bytes_to_hash(external_bytes)).into_value(external_bytes)
    }

    public fun move_from_external_storage<T: drop + store, V: store>(external_bytes: vector<u8>, witness: &V): T acquires ExternalUniqueState {
        let external_state = borrow_global_mut<ExternalUniqueState<T, V>>(@aptos_framework);
        assert!(witness == &external_state.witness, error::invalid_argument(EWITNESS_MISMATCH));
        external_state.values.remove(verified_external_value::bytes_to_hash(external_bytes)).into_value(external_bytes)
    }

    public(friend) fun borrow_mut<T: drop + store, V: store>(external_bytes: vector<u8>, witness: &V): MutableHandle<T> acquires ExternalUniqueState {
        let typed_value = move_from_external_storage<T, V>(external_bytes, witness);
        MutableHandle {
            typed_value,
        }
    }

    public(friend) fun handle_get_mut_value<T: drop + store>(self: &mut MutableHandle<T>): &mut T {
        &mut self.typed_value
    }

    public(friend) fun handle_store<T: drop + store, V: store>(self: MutableHandle<T>, witness: &V): u256 acquires ExternalUniqueState {
        let MutableHandle {
            typed_value,
        } = self;
        move_to_external_storage(typed_value, witness)
    }
}
