/// Ring buffer implementation of a double-ended queue.
///
/// Has O(1) push/pop operations at both front and back, unlike a vector
/// which has O(n) push/pop operations at the front.
module ring_deque::ring_deque {

    use std::option::{Self, Option};
    use std::vector;

    /// Buffer capacity must be nonzero.
    const E_CAPACITY_ZERO: u64 = 0;
    /// Ring deque is full;
    const E_FULL: u64 = 1;
    /// Ring deque is empty;
    const E_EMPTY: u64 = 2;

    /// Ring buffer implementation of a double-ended queue. Mechanics:
    ///
    /// 1. Uses a single vector of options to store elements, instantiated to full capacity.
    /// 2. Options are used to extract elements, but not resize the vector.
    /// 3. Bounds are maintained by front and back indices, which wrap around.
    struct RingDeque<T> has copy, drop, store {
        data: vector<Option<T>>,
        capacity: u64,
        front: u64,
        back: u64,
        length: u64
    }

    public fun new<T>(capacity: u64): RingDeque<T> {
        assert!(capacity > 0, E_CAPACITY_ZERO);
        let i = 0;
        let data = vector::empty();
        while (i < capacity) {
            vector::push_back(&mut data, option::none());
            i = i + 1;
        };
        RingDeque { data, capacity, front: 0, back: 0, length: 0 }
    }

    public fun capacity<T>(rd_ref: &RingDeque<T>): u64 { rd_ref.capacity }

    public fun length<T>(rd_ref: &RingDeque<T>): u64 { rd_ref.length }

    public fun is_empty<T>(rd_ref: &RingDeque<T>): bool { rd_ref.length == 0 }

    public fun is_full<T>(rd_ref: &RingDeque<T>): bool {
        rd_ref.length == rd_ref.capacity
    }

    public fun borrow_front<T>(rd_ref: &RingDeque<T>): &T {
        assert!(rd_ref.length > 0, E_EMPTY);
        option::borrow(vector::borrow(&rd_ref.data, rd_ref.front))
    }

    public fun borrow_front_mut<T>(rd_ref_mut: &mut RingDeque<T>): &mut T {
        assert!(rd_ref_mut.length > 0, E_EMPTY);
        option::borrow_mut(
            vector::borrow_mut(&mut rd_ref_mut.data, rd_ref_mut.front)
        )
    }

    public fun borrow_back<T>(rd_ref: &RingDeque<T>): &T {
        assert!(rd_ref.length > 0, E_EMPTY);
        option::borrow(vector::borrow(&rd_ref.data, rd_ref.back))
    }

    public fun borrow_back_mut<T>(rd_ref_mut: &mut RingDeque<T>): &mut T {
        assert!(rd_ref_mut.length > 0, E_EMPTY);
        option::borrow_mut(
            vector::borrow_mut(&mut rd_ref_mut.data, rd_ref_mut.back)
        )
    }

    public fun push_front<T>(rd_ref_mut: &mut RingDeque<T>, value: T) {
        assert!(rd_ref_mut.length < rd_ref_mut.capacity, E_FULL);
        if (rd_ref_mut.length > 0) {
            let max_index = rd_ref_mut.capacity - 1;
            rd_ref_mut.front = if (rd_ref_mut.front == 0) {
                max_index
            } else {
                rd_ref_mut.front - 1
            };
        };
        option::fill(
            vector::borrow_mut(&mut rd_ref_mut.data, rd_ref_mut.front),
            value
        );
        rd_ref_mut.length = rd_ref_mut.length + 1;
    }

    public fun push_back<T>(rd_ref_mut: &mut RingDeque<T>, value: T) {
        assert!(rd_ref_mut.length < rd_ref_mut.capacity, E_FULL);
        if (rd_ref_mut.length > 0) {
            let max_index = rd_ref_mut.capacity - 1;
            rd_ref_mut.back = if (rd_ref_mut.back == max_index) {
                0
            } else {
                rd_ref_mut.back + 1
            };
        };
        option::fill(
            vector::borrow_mut(&mut rd_ref_mut.data, rd_ref_mut.back),
            value
        );
        rd_ref_mut.length = rd_ref_mut.length + 1;
    }

    public fun pop_front<T>(rd_ref_mut: &mut RingDeque<T>): T {
        assert!(rd_ref_mut.length > 0, E_EMPTY);
        let val = option::extract(
            vector::borrow_mut(&mut rd_ref_mut.data, rd_ref_mut.front)
        );
        if (rd_ref_mut.length > 1) {
            let max_index = rd_ref_mut.capacity - 1;
            rd_ref_mut.front = if (rd_ref_mut.front == max_index) {
                0
            } else {
                rd_ref_mut.front + 1
            };
        };
        rd_ref_mut.length = rd_ref_mut.length - 1;
        val
    }

    public fun pop_back<T>(rd_ref_mut: &mut RingDeque<T>): T {
        assert!(rd_ref_mut.length > 0, E_EMPTY);
        let val = option::extract(
            vector::borrow_mut(&mut rd_ref_mut.data, rd_ref_mut.back)
        );
        if (rd_ref_mut.length > 1) {
            let max_index = rd_ref_mut.capacity - 1;
            rd_ref_mut.back = if (rd_ref_mut.back == 0) {
                max_index
            } else {
                rd_ref_mut.back - 1
            };
        };
        rd_ref_mut.length = rd_ref_mut.length - 1;
        val
    }

    #[test]
    #[expected_failure(abort_code = E_CAPACITY_ZERO)]
    fun test_new_capacity_zero() {
        new<u8>(0);
    }

    #[test]
    #[expected_failure(abort_code = E_EMPTY)]
    fun test_borrow_front_empty() {
        let rd = new<u8>(1);
        borrow_front(&rd);
    }

    #[test]
    #[expected_failure(abort_code = E_EMPTY)]
    fun test_borrow_front_mut_empty() {
        let rd = new<u8>(1);
        borrow_front_mut(&mut rd);
    }

    #[test]
    #[expected_failure(abort_code = E_EMPTY)]
    fun test_borrow_back_empty() {
        let rd = new<u8>(1);
        borrow_back(&rd);
    }

    #[test]
    #[expected_failure(abort_code = E_EMPTY)]
    fun test_borrow_back_mut_empty() {
        let rd = new<u8>(1);
        borrow_back_mut(&mut rd);
    }

    #[test]
    #[expected_failure(abort_code = E_FULL)]
    fun test_push_front_full() {
        let rd = new<u8>(1);
        push_front(&mut rd, 1);
        push_front(&mut rd, 1);
    }

    #[test]
    #[expected_failure(abort_code = E_FULL)]
    fun test_push_back_full() {
        let rd = new<u8>(1);
        push_back(&mut rd, 1);
        push_back(&mut rd, 1);
    }

    #[test]
    #[expected_failure(abort_code = E_EMPTY)]
    fun test_pop_front_empty() {
        let rd = new<u8>(1);
        pop_front(&mut rd);
    }

    #[test]
    #[expected_failure(abort_code = E_EMPTY)]
    fun test_pop_back_empty() {
        let rd = new<u8>(1);
        pop_back(&mut rd);
    }

    #[test]
    fun test_end_to_end() {
        let capacity = 5;
        let rd = new<u8>(capacity);
        // Value: | | | | | |
        // Front:  ^
        // Back:   ^
        assert!(capacity(&rd) == capacity, 0);
        assert!(length(&rd) == 0, 0);
        assert!(is_empty(&rd), 0);
        assert!(!is_full(&rd), 0);
        push_back(&mut rd, 9);
        // Value: |9| | | | |
        // Front:  ^
        // Back:   ^
        assert!(capacity(&rd) == capacity, 0);
        assert!(length(&rd) == 1, 0);
        assert!(!is_empty(&rd), 0);
        push_front(&mut rd, 7);
        // Value: |9| | | |7|
        // Front:          ^
        // Back:   ^
        push_back(&mut rd, 6);
        // Value: |9|6| | |7|
        // Front:          ^
        // Back:     ^
        *borrow_back_mut(&mut rd) = 8;
        // Value: |9|8| | |7|
        // Front:          ^
        // Back:     ^
        assert!(rd.front == 4, 0);
        assert!(rd.back == 1, 0);
        *borrow_front_mut(&mut rd) = 5;
        // Value: |9|8| | |5|
        // Front:          ^
        // Back:     ^
        assert!(pop_front(&mut rd) == 5, 0);
        // Value: |9|8| | | |
        // Front:  ^
        // Back:     ^
        assert!(*borrow_front(&rd) == 9, 0);
        assert!(*borrow_back(&rd) == 8, 0);
        assert!(pop_front(&mut rd) == 9, 0);
        // Value: | |8| | | |
        // Front:    ^
        // Back:     ^
        assert!(pop_back(&mut rd) == 8, 0);
        // Value: | | | | | |
        // Front:    ^
        // Back:     ^
        push_front(&mut rd, 5);
        push_front(&mut rd, 4);
        push_front(&mut rd, 3);
        push_back(&mut rd, 6);
        push_back(&mut rd, 7);
        // Value: |4|5|6|7|3|
        // Front:          ^
        // Back:         ^
        assert!(rd.front == 4, 0);
        assert!(rd.back == 3, 0);
        assert!(length(&rd) == 5, 0);
        assert!(is_full(&rd), 0);
        assert!(pop_back(&mut rd) == 7, 0);
        assert!(pop_back(&mut rd) == 6, 0);
        assert!(pop_back(&mut rd) == 5, 0);
        assert!(pop_back(&mut rd) == 4, 0);
        assert!(pop_back(&mut rd) == 3, 0);
        // Value: | | | | | |
        // Front:          ^
        // Back:           ^
        push_back(&mut rd, 1);
        push_back(&mut rd, 2);
        push_back(&mut rd, 3);
        // Value: |2|3| | |1|
        // Front:          ^
        // Back:     ^
        assert!(length(&rd) == 3, 0);
        assert!(pop_front(&mut rd) == 1, 0);
        assert!(pop_front(&mut rd) == 2, 0);
        assert!(pop_front(&mut rd) == 3, 0);
        // Value: | | | | | |
        // Front:    ^
        // Back:     ^
        assert!(rd.front == 1, 0);
        assert!(rd.back == 1, 0);
        assert!(length(&rd) == 0, 0);
        assert!(is_empty(&rd), 0);
    }

}
