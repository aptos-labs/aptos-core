module aptos_experimental::order_id_generation {
    use aptos_trading::order_book_types::{OrderId, new_order_id_type};
    use aptos_framework::transaction_context;

    public fun next_order_id(): OrderId {
        // reverse bits to make order ids random, so indices on top of them are shuffled.
        new_order_id_type(reverse_bits(
            transaction_context::monotonically_increasing_counter()
        ))
    }

    /// Reverse the bits in a u128 value using divide and conquer approach
    /// This is more efficient than the bit-by-bit approach, reducing from O(n) to O(log n)
    fun reverse_bits(value: u128): u128 {
        let v = value;

        // Swap odd and even bits
        v =
            ((v & 0x55555555555555555555555555555555) << 1)
                | ((v >> 1) & 0x55555555555555555555555555555555);

        // Swap consecutive pairs
        v =
            ((v & 0x33333333333333333333333333333333) << 2)
                | ((v >> 2) & 0x33333333333333333333333333333333);

        // Swap nibbles
        v =
            ((v & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f) << 4)
                | ((v >> 4) & 0x0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f);

        // Swap bytes
        v =
            ((v & 0x00ff00ff00ff00ff00ff00ff00ff00ff) << 8)
                | ((v >> 8) & 0x00ff00ff00ff00ff00ff00ff00ff00ff);

        // Swap 2-byte chunks
        v =
            ((v & 0x0000ffff0000ffff0000ffff0000ffff) << 16)
                | ((v >> 16) & 0x0000ffff0000ffff0000ffff0000ffff);

        // Swap 4-byte chunks
        v =
            ((v & 0x00000000ffffffff00000000ffffffff) << 32)
                | ((v >> 32) & 0x00000000ffffffff00000000ffffffff);

        // Swap 8-byte chunks
        v = (v << 64) | (v >> 64);

        v
    }

    // ============================= Tests ====================================

    #[test]
    fun test_reverse_bits_order_id_type() {
        // Test basic bit reversal functionality
        let order_id_1 = 1;
        let order_id_2 = 2;
        let order_id_3 = 0x12345678;
        let order_id_4 = 0x87654321ABCDEF00;

        let reversed_1 = reverse_bits(order_id_1);
        let reversed_2 = reverse_bits(order_id_2);
        let reversed_3 = reverse_bits(order_id_3);
        let reversed_4 = reverse_bits(order_id_4);

        // Test that conversion back gives original value
        let recovered_1 = reverse_bits(reversed_1);
        let recovered_2 = reverse_bits(reversed_2);
        let recovered_3 = reverse_bits(reversed_3);
        let recovered_4 = reverse_bits(reversed_4);

        assert!(order_id_1 == recovered_1);
        assert!(order_id_2 == recovered_2);
        assert!(order_id_3 == recovered_3);
        assert!(order_id_4 == recovered_4);

        // Test that reversed values are different from originals (for non-palindromic bit patterns)
        // Now we can access the internal field since we're in the same module
        assert!(reversed_1 != order_id_1);
        assert!(reversed_2 != order_id_2);
        assert!(reversed_3 != order_id_3);
        assert!(reversed_4 != order_id_4);

        // Test specific bit reversal cases
        // 1 in binary: 0...0001, reversed should be 1000...0000 (high bit set)
        assert!(reversed_1 == (1u128 << 127));

        // 2 in binary: 0...0010, reversed should be 0100...0000
        assert!(reversed_2 == (1u128 << 126));

        // Test edge cases
        let order_id_zero = 0;
        let reversed_zero = reverse_bits(order_id_zero);
        let recovered_zero = reverse_bits(reversed_zero);
        assert!(order_id_zero == recovered_zero);
        assert!(reversed_zero == 0); // 0 reversed is still 0

        // Test maximum value
        let order_id_max = 0xffffffffffffffffffffffffffffffff;
        let reversed_max = reverse_bits(order_id_max);
        let recovered_max = reverse_bits(reversed_max);
        assert!(order_id_max == recovered_max);
        assert!(reversed_max == 0xffffffffffffffffffffffffffffffff); // All 1s reversed is still all 1s

        // Test alternating pattern
        let order_id_alt = 0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;
        let reversed_alt = reverse_bits(order_id_alt);
        let recovered_alt = reverse_bits(reversed_alt);
        assert!(order_id_alt == recovered_alt);
        // 0xaaaa... in binary is 10101010..., reversed should be 01010101... = 0x5555...
        assert!(reversed_alt == 0x55555555555555555555555555555555);

        let order_id_alt = 0x64328946124712951320956108326756;
        let reversed_alt = reverse_bits(order_id_alt);
        let recovered_alt = reverse_bits(reversed_alt);
        assert!(order_id_alt == recovered_alt);
    }

}
