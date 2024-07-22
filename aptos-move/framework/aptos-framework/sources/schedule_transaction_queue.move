module aptos_framework::schedule_transaction_queue {
    use std::bcs;
    use std::option;
    use std::signer;
    use std::vector;
    use std::hash::sha3_256;
    use aptos_std::iterable_table;
    use aptos_std::iterable_table::IterableTable;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_framework::aggregator_v2::{Self, Aggregator};
    use aptos_framework::avl_queue::{Self, AVLqueue};
    use aptos_framework::system_addresses;
    use aptos_framework::transaction_context::EntryFunctionPayload;

    friend aptos_framework::transaction_validation;
    friend aptos_framework::block;

    struct ScheduledTransaction has copy, drop, store {
        // with a granularity of 1 second
        scheduled_time: u64,
        max_gas_unit: u64,
        sender: address,
        payload: EntryFunctionPayload,
    }

    struct TransactionId has copy, drop, store {
        hash: vector<u8>,
    }

    struct ScheduledQueue has key {
        queue: AVLqueue<IterableTable<TransactionId, bool /* placeholder for unit () */>>,
        items: TableWithLength<TransactionId, ScheduledTransaction>,
    }

    struct ToRemove has key {
        num: Aggregator<u64>,
    }

    public fun new_transaction(scheduled_time: u64, max_gas_unit: u64, payload: EntryFunctionPayload, sender: address): ScheduledTransaction {
        // todo:: validate payload
        ScheduledTransaction {
            scheduled_time: scheduled_time,
            max_gas_unit,
            sender,
            payload,
        }
    }

    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        move_to(framework, ScheduledQueue {
            queue: avl_queue::new(true, 0, 0),
            items: table_with_length::new(),
        });
        move_to(framework, ToRemove {
            num: aggregator_v2::create_unbounded_aggregator(),
        });
    }

    public fun insert(sender: &signer, txn: ScheduledTransaction) acquires ScheduledQueue {
        assert!(signer::address_of(sender) == txn.sender, 1);
        let scheduled_queue = borrow_global_mut<ScheduledQueue>(@aptos_framework);
        let id = TransactionId { hash: sha3_256(bcs::to_bytes(&txn)) };
        if (table_with_length::contains(&scheduled_queue.items, id)) {
            return
        };
        // assert timestamp range
        let time = txn.scheduled_time;
        if (!avl_queue::has_key(&scheduled_queue.queue, time)) {
            avl_queue::insert(&mut scheduled_queue.queue, time, iterable_table::new());
        };
        let (node_id, _) = avl_queue::search(&scheduled_queue.queue, time);
        // Number of bits list node ID is shifted in an access key.
        // const SHIFT_ACCESS_LIST_NODE_ID: u8 = 33;
        let access_key = node_id << 33;
        iterable_table::add(
            avl_queue::borrow_mut(&mut scheduled_queue.queue, access_key), id, false);
        table_with_length::add(&mut scheduled_queue.items, id, txn);
    }

    public fun cancel(sender: address, txn_id: vector<u8>) acquires ScheduledQueue {
        let scheduled_queue = borrow_global_mut<ScheduledQueue>(@aptos_framework);
        let id = TransactionId { hash: txn_id };
        if (!table_with_length::contains(&scheduled_queue.items, id)) {
            return
        };
        let item = table_with_length::remove(&mut scheduled_queue.items, id);
        if (item.sender != sender) {
            table_with_length::add(&mut scheduled_queue.items, id, item);
        } else {
            iterable_table::remove(avl_queue::borrow_mut(
                &mut scheduled_queue.queue, item.scheduled_time), id);
            if (iterable_table::length(avl_queue::borrow(&scheduled_queue.queue, item.scheduled_time)) == 0) {
                let empty_table = avl_queue::remove(&mut scheduled_queue.queue, item.scheduled_time);
                iterable_table::destroy_empty(empty_table);
            }
        }
    }

    // Execute view function before execution to prepare scheduled transaction (pop head is fine since the side effect is not persisted)
    #[view]
    public fun get_ready_transactions(timestamp: u64, limit: u64): vector<ScheduledTransaction> acquires ScheduledQueue, ToRemove {
        reset();
        let scheduled_queue = borrow_global_mut<ScheduledQueue>(@aptos_framework);
        let result = vector[];
        while (vector::length(&result) < limit) {
            let head_key = avl_queue::get_head_key(&scheduled_queue.queue);
            if (option::is_none(&head_key)) {
                return result
            };
            let current_timestamp = option::extract(&mut head_key);
            if (current_timestamp > timestamp) {
                return result
            };
            let table = avl_queue::pop_head(&mut scheduled_queue.queue);
            let key = iterable_table::head_key(&table);
            while (option::is_some(&key)) {
                if (vector::length(&result) < limit) {
                    let txn = table_with_length::borrow(&scheduled_queue.items, *option::borrow(&key));
                    vector::push_back(&mut result, *txn);
                };
                let (_, _, next) = iterable_table::remove_iter(&mut table, *option::borrow(&key));
                key = next;
            };
            iterable_table::destroy_empty(table);
        };
        result
    }

    /// Increment at every scheduled transaction without affect parallelism
    public(friend) fun finish_execution() acquires ToRemove {
        let to_remove = borrow_global_mut<ToRemove>(@aptos_framework);
        aggregator_v2::add(&mut to_remove.num, 1);
    }

    /// Reset at beginning of each block
    public(friend) fun reset() acquires ToRemove, ScheduledQueue {
        let to_remove = borrow_global_mut<ToRemove>(@aptos_framework);
        let num_to_remove = aggregator_v2::read(&to_remove.num);
        aggregator_v2::sub(&mut to_remove.num, num_to_remove);
        let scheduled_queue = borrow_global_mut<ScheduledQueue>(@aptos_framework);
        while (num_to_remove > 0) {
            let head_key = option::extract(&mut avl_queue::get_head_key(&scheduled_queue.queue));
            let table = avl_queue::pop_head(&mut scheduled_queue.queue);
            let key = iterable_table::head_key(&table);
            while (option::is_some(&key) && num_to_remove > 0) {
                table_with_length::remove(&mut scheduled_queue.items, *option::borrow(&key));
                let (_, _, next) = iterable_table::remove_iter(&mut table, *option::borrow(&key));
                key = next;
                num_to_remove = num_to_remove - 1;
            };
            if (option::is_none(&key)) {
                iterable_table::destroy_empty(table);
            } else {
                avl_queue::insert(&mut scheduled_queue.queue, head_key, table);
                return
            }
        }
    }

    #[test(fx = @0x1)]
    fun test_insert(fx: &signer) acquires ToRemove, ScheduledQueue {
        initialize(fx);
        let txn = new_transaction(100, 1000, transaction_context::new_entry_function_payload(@0x1, string::utf8(b"foo"), string::utf8(b"bar"), vector[], vector[]), signer::address_of(fx));
        insert(fx, txn);
        assert!(vector::length(&get_ready_transactions(100, 1)) == 1, 1);
    }
}
