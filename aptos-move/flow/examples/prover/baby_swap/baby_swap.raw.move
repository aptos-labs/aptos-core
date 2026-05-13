module 0x42::baby_swap {

    // ==================== Pool State ====================

    struct Pool has key {
        x_reserve: u64,
        y_reserve: u64,
    }

    // ==================== Pool Setup ====================

    // Create a new pool with initial reserves.
    fun create_pool(account: &signer, x: u64, y: u64) {
        assert!(x > 0 && y > 0, 0);
        move_to(account, Pool { x_reserve: x, y_reserve: y });
    }


    // ==================== Swap Output ====================

    // Constant-product output: dy = dx * ry / (rx + dx).
    // Uses u128 intermediate to avoid overflow on dx * ry.
    fun output_amount(dx: u64, rx: u64, ry: u64): u64 {
        let num = (dx as u128) * (ry as u128);
        let den = (rx as u128) + (dx as u128);
        ((num / den) as u64)
    }

    // ==================== Single Swap ====================

    // Swap dx of X into the pool, receive dy of Y.
    fun swap(pool_addr: address, dx: u64): u64 {
        assert!(dx > 0, 1);
        let pool = &mut Pool[pool_addr];
        let dy = output_amount(dx, pool.x_reserve, pool.y_reserve);
        assert!(dy > 0, 2);
        pool.x_reserve = pool.x_reserve + dx;
        pool.y_reserve = pool.y_reserve - dy;
        dy
    }


    // ==================== Batch Swap ====================

    // Execute n swaps of size dx against the pool.
    // Returns total Y received across all swaps.
    // Each successive swap gets a worse price because
    // X reserve grows and Y reserve shrinks.
    fun batch_swap(pool_addr: address, dx: u64, n: u64): u64 {
        let rx0 = Pool[pool_addr].x_reserve;
        let ry0 = Pool[pool_addr].y_reserve;
        let total_dy = 0u64;
        let i = 0u64;
        while (i < n) {
            let dy = swap(pool_addr, dx);
            total_dy = total_dy + dy;
            i = i + 1;
        };
        total_dy
    }
}
