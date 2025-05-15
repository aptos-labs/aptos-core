// Copyright Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0xABCD::collections {
    // use std::signer;
    // use std::vector;
    // use aptos_std::object;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::big_ordered_map::{Self, BigOrderedMap};

    /// The caller tried to mutate an item outside the bounds of the vector.
    const E_INDEX_OUT_OF_BOUNDS: u64 = 1;

    struct None has drop, copy, store { }

    struct SmartTableNoneResource has key {
        map: SmartTable<u64, None>,
    }

    struct BigOrderedMapNoneResource has key {
        map: BigOrderedMap<u64, None>
    }

    struct VariableBigOrderedMapResource has key {
        map: BigOrderedMap<vector<u8>, vector<u8>>
    }

    fun init_module(publisher: &signer) {
        move_to<SmartTableNoneResource>(
            publisher,
            SmartTableNoneResource {
                map: smart_table::new(),
                // map: smart_table::new_with_config(0, 75, 32 * 4),
            }
        );
        move_to<BigOrderedMapNoneResource>(
            publisher,
            BigOrderedMapNoneResource {
                map: big_ordered_map::new(),
                // map: big_ordered_map::new_with_config(32 * 4, 32 * 4, true, 0),
                // map: big_ordered_map::new_with_config(64, 256, true, 0),
            }
        );
        move_to<VariableBigOrderedMapResource>(
            publisher,
            VariableBigOrderedMapResource {
                // for testing set to largest default variable degree
                map: big_ordered_map::new_with_config((409600 / 5120) as u16, (409600 / 5120 / 2) as u16, false),
            }
        );

        // let count = 20;
        // for (i in 0..count) {
        //     insert_none_into_smart_table(4_294_967_295 * i / count);
        //     insert_none_into_btree_map(4_294_967_295 * i / count);
        // };
    }

    entry fun insert_none_into_smart_table(keys: vector<u64>) acquires SmartTableNoneResource {
        let resource = borrow_global_mut<SmartTableNoneResource>(@publisher_address);

        keys.for_each_reverse(|key| {
            resource.map.upsert(key, None {});
        });
    }

    entry fun insert_none_into_btree_map(keys: vector<u64>) acquires BigOrderedMapNoneResource {
        let resource = borrow_global_mut<BigOrderedMapNoneResource>(@publisher_address);

        keys.for_each_reverse(|key| {
            resource.map.upsert(key, None {});
        });
    }

    entry fun insert_variable_into_btree_map(keys: vector<vector<u8>>, values: vector<vector<u8>>) acquires VariableBigOrderedMapResource {
        let resource = borrow_global_mut<VariableBigOrderedMapResource>(@publisher_address);

        while (!keys.is_empty() && !values.is_empty()) {
            resource.map.upsert(keys.pop_back(), values.pop_back());
        };
    }
}
