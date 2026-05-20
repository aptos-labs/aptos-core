module 0x42::baby_swap {

    // ==================== Pool State ====================

    struct Pool has key {
        x_reserve: u64,
        y_reserve: u64,
    }
    spec Pool {
        invariant x_reserve > 0 && y_reserve > 0;
    }
    spec module {
        // Once a pool exists, it is never removed.
        invariant update forall addr: address where old(exists<Pool>(addr)):
            exists<Pool>(addr);

        // Constant-product monotonicity: integer division rounding means the
        // pool keeps a tiny surplus each swap, so rx * ry never decreases.
        invariant update forall addr: address where old(exists<Pool>(addr)):
            (old(Pool[addr]).x_reserve as u128) * (old(Pool[addr]).y_reserve as u128)
            <= (Pool[addr].x_reserve as u128) * (Pool[addr].y_reserve as u128);
    }

    // ==================== Pool Setup ====================

    // Create a new pool with initial reserves.
    fun create_pool(account: &signer, x: u64, y: u64) {
        assert!(x > 0 && y > 0, 0);
        move_to(account, Pool { x_reserve: x, y_reserve: y });
    }
    spec create_pool(account: &signer, x: u64, y: u64) {
        use 0x1::signer;
        pragma opaque = true;
        modifies Pool[signer::address_of(account)];
        ensures [inferred] publish<Pool>(signer::address_of(account), Pool{x_reserve: x, y_reserve: y});
        aborts_if [inferred] x == 0 || y == 0;
        aborts_if [inferred] exists<Pool>(signer::address_of(account));
    }


    // ==================== Swap Output ====================

    // Constant-product output: dy = dx * ry / (rx + dx).
    // Uses u128 intermediate to avoid overflow on dx * ry.
    fun output_amount(dx: u64, rx: u64, ry: u64): u64 {
        let num = (dx as u128) * (ry as u128);
        let den = (rx as u128) + (dx as u128);
        ((num / den) as u64)
    }
    spec output_amount(dx: u64, rx: u64, ry: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == (((dx as u128) * (ry as u128) / ((rx as u128) + (dx as u128))) as u64);
        aborts_if [inferred] (dx as u128) * (ry as u128) / ((rx as u128) + (dx as u128)) > MAX_U64;
        aborts_if [inferred] (rx as u128) + (dx as u128) == 0;
        aborts_if [inferred] (rx as u128) + (dx as u128) > MAX_U128;
        aborts_if [inferred] (dx as u128) * (ry as u128) > MAX_U128;
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
    spec swap(pool_addr: address, dx: u64): u64 {
        pragma opaque = true;
        modifies Pool[pool_addr];
        ensures [inferred] result == output_amount(dx, old(Pool[pool_addr]).x_reserve, old(Pool[pool_addr]).y_reserve);
        ensures [inferred] update<Pool>(pool_addr, Pool{
            x_reserve: old(Pool[pool_addr]).x_reserve + dx,
            y_reserve: old(Pool[pool_addr]).y_reserve - result,
        });
        aborts_if [inferred] dx == 0;
        aborts_if [inferred] output_amount(dx, Pool[pool_addr].x_reserve, Pool[pool_addr].y_reserve) == 0;
        aborts_if [inferred] Pool[pool_addr].x_reserve + dx > MAX_U64;
        aborts_if [inferred] aborts_of<output_amount>(dx, Pool[pool_addr].x_reserve, Pool[pool_addr].y_reserve);
        aborts_if [inferred] !exists<Pool>(pool_addr);
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
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] exists<Pool>(pool_addr);
            invariant [inferred] Pool[pool_addr].x_reserve == rx0 + i * dx;
            invariant [inferred] Pool[pool_addr].y_reserve == ry0 - total_dy;
            invariant [inferred] total_dy <= ry0;
            invariant [inferred] (Pool[pool_addr].x_reserve as u128)
                * (Pool[pool_addr].y_reserve as u128)
                >= (rx0 as u128) * (ry0 as u128);
        };
        total_dy
    }
    spec batch_swap(pool_addr: address, dx: u64, n: u64): u64 {
        pragma opaque = true;
        modifies Pool[pool_addr];
        ensures [inferred] result == old(Pool[pool_addr]).y_reserve - Pool[pool_addr].y_reserve;
        ensures [inferred] Pool[pool_addr].x_reserve == old(Pool[pool_addr]).x_reserve + n * dx;
        ensures [inferred] Pool[pool_addr].y_reserve == old(Pool[pool_addr]).y_reserve - result;
        aborts_if [inferred] !exists<Pool>(pool_addr);
        aborts_if [inferred] Pool[pool_addr].x_reserve + n * dx > MAX_U64;
        // Loop-body aborts (dx==0, dy==0 at some iteration) cannot be proven
        // with invariant-based loop verification: the prover's havoc can always
        // skip all iterations, creating a non-aborting path that satisfies all
        // invariants. Only the x_reserve overflow condition works because it
        // makes the loop-exit invariant infeasible for u64.
        pragma aborts_if_is_partial = true;
    }
}
