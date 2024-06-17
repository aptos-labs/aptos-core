module aptos_framework::schedule_transaction_queue {
    use std::bcs;
    use std::signer;
    use aptos_std::aptos_hash::sha3_512;
    use aptos_std::iterable_table;
    use aptos_std::iterable_table::IterableTable;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_framework::avl_queue::{Self, AVLqueue};
    use aptos_framework::system_addresses;

    struct ScheduledTransaction has copy, drop, store {
        // with a granularity of 1 second
        scheduled_time: u64,
        payload: vector<u8>,
        sender: address,
    }

    struct TransactionId has copy, drop, store {
        hash: vector<u8>,
    }

    struct ScheduledQueue has key {
        queue: AVLqueue<IterableTable<TransactionId, bool /* placeholder for unit () */>>,
        items: TableWithLength<TransactionId, ScheduledTransaction>,
    }

    public fun initialize(framework: signer) {
        system_addresses::assert_aptos_framework(&framework);
        move_to(&framework, ScheduledQueue {
            queue: avl_queue::new(true, 0, 0),
            items: table_with_length::new(),
        });
    }

    fun insert(sender: &signer, txn: ScheduledTransaction) acquires ScheduledQueue {
        assert!(signer::address_of(sender) == txn.sender, 1);
        let scheduled_queue = borrow_global_mut<ScheduledQueue>(@aptos_framework);
        let id = TransactionId { hash: sha3_512(bcs::to_bytes(&txn)) };
        if (!table_with_length::contains(&scheduled_queue.items, id)) {
            return
        };
        // assert timestamp range
        let time = txn.scheduled_time;
        if (!avl_queue::has_key(&scheduled_queue.queue, time)) {
            avl_queue::insert(&mut scheduled_queue.queue, time, iterable_table::new());
        };
        iterable_table::add(
            avl_queue::borrow_mut(&mut scheduled_queue.queue, time), id, false);
        table_with_length::add(&mut scheduled_queue.items, id, txn);
    }

    fun cancel(sender: address, txn_id: vector<u8>) acquires ScheduledQueue {
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
}
