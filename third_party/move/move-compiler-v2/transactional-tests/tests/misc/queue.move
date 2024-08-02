//# publish
module 0x42::queue {
    use std::vector;

    struct Queue<T> has key, drop { data: vector<T> }

    public fun create<T>(): Queue<T> {
        Queue { data: vector::empty() }
    }

    public fun enqueue<T>(queue: &mut Queue<T>, item: T) {
        vector::push_back(&mut queue.data, item);
    }

    public fun dequeue<T>(queue: &mut Queue<T>): T {
        let item = vector::remove(&mut queue.data, 0);
        item
    }

    public fun test_queue_operations() {
        let queue = create();
        enqueue(&mut queue, 40);
        enqueue(&mut queue, 41);
        enqueue(&mut queue, 42);
        assert!(dequeue(&mut queue) == 40, 1);
        enqueue(&mut queue, 43);
        assert!(dequeue(&mut queue) == 41, 2);
        assert!(dequeue(&mut queue) == 42, 3);
        assert!(dequeue(&mut queue) == 43, 4);
    }
}

//# run 0x42::queue::test_queue_operations
