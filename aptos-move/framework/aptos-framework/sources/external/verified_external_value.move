/// Module for ability to take move values, and store them externally
/// (and save on the cost difference between onchain and offchain state),
/// with ability to safely retrieve them later: to safely transorm them back
/// to move value, by providing the externally stored bytes.
///
/// We do so by keeping the hash of the value in the onchain state, used to verify validity
/// of external bytes, guaranteeing validity of deserialized value.
module aptos_framework::verified_external_value {
    const EHASH_DOESNT_MATCH: u64 = 1;
    const EHASH_DOESNT_EXIST: u64 = 1;

    // TODO replace with BigOrderedMap
    use std::bcs;
    use std::error;
    use std::hash;
    use aptos_std::from_bcs;
    use aptos_std::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::event;
    use aptos_framework::util;

    /// Externally storing any move value, while keeping the hash inside ExternalValue.
    ///
    /// Currently we require value to have drop - as value dissapears from chain
    struct ExternalValue<phantom T: drop + store> has drop, store {
        hash: u256,
    }

    struct Dummy has drop, store { }

    /// Set of `ExternalValue`s, where only their hashes are stored onchain.
    /// Set is without duplicates - i.e. same value cannot be stored twice.
    struct ExternalValuesSet<phantom T: drop + store> has store {
        hashes: BigOrderedMap<u256, Dummy>,
    }

    #[event]
    struct MovedToExternalStorage<phantom T: drop + store> has drop, store {
        hash: u256,
        // bytes needed to be passed back, in order to extract value.
        bytes: vector<u8>,
    }

    /// Takes a value, emits it as an event, and creates ExternalValue representing it
    /// (which stores it's hash inside)
    public fun move_to_external_storage<T: drop + store>(value: T): ExternalValue<T> {
        let bytes = bcs::to_bytes(&value);
        let hash = bytes_to_hash(bytes);

        event::emit(MovedToExternalStorage<T> {
            hash,
            bytes,
        });
        ExternalValue {
            hash,
        }
    }

    /// Retrieves the hash of the value ExternalValue represents.
    public fun get_hash<T: drop + store>(self: &ExternalValue<T>): u256 {
        self.hash
    }

    /// Converts `ExternalValue` into it's original move representation, by providing it's bytes.
    public fun into_value<T: drop + store>(self: ExternalValue<T>, external_bytes: vector<u8>): T {
        let ExternalValue { hash } = self;
        let data_hash = bytes_to_hash(external_bytes);
        assert!(data_hash == hash, error::invalid_argument(EHASH_DOESNT_MATCH));

        // maybe emit consumed event, so indexer can remove storing it?

        util::from_bytes<T>(external_bytes)
    }


    /// For a type that has `copy`, return original move representation of `ExternalValue`, without consuming it.
    public fun get_value_copy<T: drop + store + copy>(self: &ExternalValue<T>, external_bytes: vector<u8>): T {
        let data_hash = bytes_to_hash(external_bytes);
        assert!(data_hash == self.hash, error::invalid_argument(EHASH_DOESNT_MATCH));
        util::from_bytes<T>(external_bytes)
    }

    /// Creates new `ExternalValuesSet`
    public fun new_set<T: drop + store>(): ExternalValuesSet<T> {
        ExternalValuesSet { hashes: big_ordered_map::new() }
    }

    /// Checks whether `ExternalValuesSet` contains `ExternalValue` with corresponding hash.
    public fun contains<T: drop + store>(self: &ExternalValuesSet<T>, hash: u256): bool {
        self.hashes.contains(&hash)
    }

    /// Adds `ExternalValue` to `ExternalValuesSet`
    /// Abort if `value` already exists.
    public fun add<T: drop + store>(self: &mut ExternalValuesSet<T>, value: ExternalValue<T>) {
        let ExternalValue { hash } = value;
        self.hashes.add(hash, Dummy {});
    }

    /// Removes and returns `ExternalValue` with given `hash` from `ExternalValuesSet`.
    /// Aborts if there is no entry for `hash`.
    public fun remove<T: drop + store>(self: &mut ExternalValuesSet<T>, hash: u256): ExternalValue<T> {
        self.hashes.remove(&hash);
        ExternalValue { hash }
    }

    /// For a type that has `copy`, returns `ExternalValue` with the given hash.
    /// Aborts if there is no entry for `hash`.
    public fun get_copy<T: drop + store + copy>(self: &ExternalValuesSet<T>, hash: u256): ExternalValue<T> {
        assert!(self.hashes.contains(&hash), error::invalid_argument(EHASH_DOESNT_EXIST));
        ExternalValue { hash }
    }

    /// Computes a hash of a given bytes.
    /// Value is first serialized using `bcs::to_bytes`, and then the hash is computed using this function.
    public fun bytes_to_hash(external_bytes: vector<u8>): u256 {
        from_bcs::to_u256(hash::sha3_256(external_bytes))
    }
}
