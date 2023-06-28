spec aptos_std::smart_table {

    spec module {
        pragma verify = false;
    }
    use aptos_std::aptos_hash::{spec_sip_hash};

    spec SmartTable {
        pragma bv = b"1,2,3";
        invariant (split_load_threshold <= (100 as u8)) && (split_load_threshold > (0 as u8));
        invariant num_buckets != (0 as u64);
        invariant level <= (62 as u8);
        invariant target_bucket_size > (0 as u64);
    }

    spec Entry {
        pragma bv = b"0";
    }

    spec destroy {
        pragma verify = true;
        requires table.num_buckets == int2bv((table_with_length::spec_len(table.buckets) as u64));
    }

    spec bucket_index {
        pragma verify = true;
        pragma bv = b"0,1,2";
        ensures ((level + (1 as u8)) as u8) <= (63 as u8) ==> ((level + (1 as u8)) as u8) > level;
        aborts_if ((level + (1 as u8)) as u8) > (63 as u8);
        aborts_if ((level + (1 as u8)) as u8) < level;
    }

    spec length {
        aborts_if false;
    }

    spec load_factor {
        pragma verify = true;
        aborts_if (table.size * (100 as u64) as u64) < table.size;
        aborts_if (table.size * (100 as u64) as u64) >= (MAX_U64 as u64) || (table.num_buckets == (0 as u64)) || (int2bv(table.target_bucket_size) == (0 as u64));
    }

    spec update_split_load_threshold {
        pragma verify = true;
        ensures table.split_load_threshold == split_load_threshold;
    }

    spec update_target_bucket_size {
        pragma verify = true;
        ensures table.target_bucket_size == target_bucket_size;
    }

    spec contains {
        use std::bcs;
        pragma verify = true;
        pragma verify_duration_estimate = 1000; // TODO: set because of timeout (property proved)
        pragma aborts_if_is_partial = true;
        let hash = spec_sip_hash(bcs::serialize(key));
        let index = hash % (1 << (bv2int(table.level) + 1));
        let idx = if (index < bv2int(table.num_buckets)) {
            index
        } else {
            index % (1 << bv2int(table.level))
        };
        aborts_if !table_with_length::spec_contains(table.buckets, idx);
    }

    spec borrow {
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)
        use std::bcs;
        pragma aborts_if_is_partial = true;
        let hash = spec_sip_hash(bcs::serialize(key));
        let index = hash % (1 << (bv2int(table.level) + 1));
        let idx = if (index < bv2int(table.num_buckets)) {
            index
        } else {
            index % (1 << bv2int(table.level))
        };
        aborts_if vector::length(table_with_length::spec_get(table.buckets, idx)) == 0;
        aborts_if forall i in 0..vector::length(table_with_length::spec_get(table.buckets,idx)):
                vector::borrow(
                    table_with_length::spec_get(
                            table.buckets, idx
                    ),
                i).key != key;
        aborts_if !table_with_length::spec_contains(table.buckets, idx);
    }

    spec remove {
        use std::bcs;
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)
        pragma aborts_if_is_partial = true;
        let hash = spec_sip_hash(bcs::serialize(key));
        let index = hash % (1 << (bv2int(table.level) + 1));
        let idx = if (index < bv2int(table.num_buckets)) {
            index
        } else {
            index % (1 << bv2int(table.level))
        };
        aborts_if table.size == (0 as u64);
        aborts_if vector::length(table_with_length::spec_get(table.buckets, idx)) == 0;
        aborts_if forall i in 0..vector::length(table_with_length::spec_get(table.buckets, idx)):
                vector::borrow(
                    table_with_length::spec_get(
                            table.buckets, idx
                    ),
                i).key != key;
        aborts_if !table_with_length::spec_contains(table.buckets, idx);
    }

    spec borrow_mut {
        use std::bcs;
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)
        pragma aborts_if_is_partial = true;
        let hash = spec_sip_hash(bcs::serialize(key));
        let index = hash % (1 << (bv2int(table.level) + 1));
        let idx = if (index < bv2int(table.num_buckets)) {
            index
        } else {
            index % (1 << bv2int(table.level))
        };
        aborts_if vector::length(table_with_length::spec_get(table.buckets, idx)) == 0;
        aborts_if !table_with_length::spec_contains(table.buckets, idx);
    }

    spec destroy_empty {
        pragma aborts_if_is_partial = true;
        aborts_if table.size != (0 as u64);
        requires forall i in 0..table_with_length::spec_len(table.buckets): table_with_length::spec_contains(table.buckets, i);
        requires forall i in 0..table_with_length::spec_len(table.buckets): vector::length(table_with_length::spec_get(table.buckets, i)) == 0;
        requires table.num_buckets == int2bv((table_with_length::spec_len(table.buckets) as u64));
    }

    // Temporary Mockup for old failng modules.
    spec split_one_bucket {
        pragma verify = false;
    }
    spec add {
        pragma verify = false;
    }
    spec new_with_config {
        pragma verify = false;
    }
    spec new {
        pragma verify = false;
    }

    spec borrow_mut_with_default {
        pragma verify = false;
    }

    spec borrow_with_default {
        pragma verify = false;
    }

    spec upsert {
        pragma verify = false;
    }

    // helper functions
    spec fun get_index<K>(level: u8, num_buckets: u64, key: K): u64;
    // {
    //     use std::bcs;
    //     let hash = spec_sip_hash(bcs::serialize(key));
    //     let index = hash % (1 << (bv2int(level) + 1));
    //     if (index < bv2int(num_buckets)) {
    //         index
    //     } else {
    //         index % (1 << bv2int(level))
    //     }
    // }

    //spec fun get_index<K>(level: u8, num_buckets: u64, key: K): u64;
    // {
    //     use std::bcs;
    //     let hash = spec_sip_hash(bcs::serialize(key));
    //     let index = hash % (1 << (level + 1));
    //     if (index < num_buckets) {
    //         index
    //     } else {
    //         index % (1 << level)
    //     }
    // }
}
