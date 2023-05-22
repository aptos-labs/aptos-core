spec aptos_std::smart_table {
    use aptos_std::aptos_hash::{spec_sip_hash};

    spec SmartTable {
        invariant split_load_threshold <= 100 && split_load_threshold > 0;
        invariant num_buckets != 0;
        invariant level <= 62;
        invariant target_bucket_size > 0;
    }

    spec destroy {
        requires table.num_buckets == table_with_length::spec_len(table.buckets);
    }

    spec bucket_index {
        aborts_if level + 1 > 63;
    }

    spec length {
        aborts_if false;
    }

    spec load_factor {
        aborts_if (table.size * 100 >= MAX_U64) || (table.num_buckets == 0) || (table.target_bucket_size == 0);
    }

    spec update_split_load_threshold {
        ensures table.split_load_threshold == split_load_threshold;
    }

    spec update_target_bucket_size {
        ensures table.target_bucket_size == target_bucket_size;
    }

    spec contains {
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)
        pragma aborts_if_is_partial = true;
        aborts_if !table_with_length::spec_contains(table.buckets, get_index(table.level, table.num_buckets, key));
    }

    spec borrow {
        pragma aborts_if_is_partial = true;
        aborts_if vector::length(table_with_length::spec_get(table.buckets, get_index(table.level, table.num_buckets, key))) == 0;
        aborts_if forall i in 0..vector::length(table_with_length::spec_get(table.buckets, get_index(table.level, table.num_buckets, key))):
                vector::borrow(
                    table_with_length::spec_get(
                            table.buckets, get_index(table.level, table.num_buckets, key)
                    ),
                i).key != key;
        aborts_if !table_with_length::spec_contains(table.buckets, get_index(table.level, table.num_buckets, key));
    }

    spec remove {
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)
        pragma aborts_if_is_partial = true;
        aborts_if table.size == 0;
        aborts_if vector::length(table_with_length::spec_get(table.buckets, get_index(table.level, table.num_buckets, key))) == 0;
        aborts_if forall i in 0..vector::length(table_with_length::spec_get(table.buckets, get_index(table.level, table.num_buckets, key))):
                vector::borrow(
                    table_with_length::spec_get(
                            table.buckets, get_index(table.level, table.num_buckets, key)
                    ),
                i).key != key;
        aborts_if !table_with_length::spec_contains(table.buckets, get_index(table.level, table.num_buckets, key));
    }

    spec borrow_mut {
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)
        pragma aborts_if_is_partial = true;
        aborts_if vector::length(table_with_length::spec_get(table.buckets, get_index(table.level, table.num_buckets, key))) == 0;
        aborts_if !table_with_length::spec_contains(table.buckets, get_index(table.level, table.num_buckets, key));
    }

    spec destroy_empty {
        pragma aborts_if_is_partial = true;
        aborts_if table.size != 0;
        requires forall i in 0..table_with_length::spec_len(table.buckets): table_with_length::spec_contains(table.buckets, i);
        requires forall i in 0..table_with_length::spec_len(table.buckets): vector::length(table_with_length::spec_get(table.buckets, i)) == 0;
        requires table.num_buckets == table_with_length::spec_len(table.buckets);
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
    spec fun get_index<K>(level: u8, num_buckets: u64, key: K): u64 {
        use std::bcs;
        let hash = spec_sip_hash(bcs::serialize(key));
        let index = hash % (1 << (level + 1));
        if (index < num_buckets) {
            index
        } else {
            index % (1 << level)
        }
    }
}
