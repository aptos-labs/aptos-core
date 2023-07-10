// flag: --vector-theory=BoogieArray
module 0x42::test {
    use std::vector;
    use extensions::table::{Self, Table};

    fun f1(pool: &mut vector<address>, addr: address) {
        let (_, idx) = vector::index_of(pool, &addr);
        vector::remove(pool, idx);
    }
    spec f1 {
        invariant forall i in 0..len(pool), j in 0..len(pool):
            (pool[i] == pool[j]) ==> (i == j);

        ensures forall a: address where a != addr:
            old(contains(pool, a)) ==> contains(pool, a);
    }

    struct Pool {
        shares: Table<address, u64>,
        holders: vector<address>,
    }

    fun f2(pool: &mut Pool, addr: address) {
        table::remove(&mut pool.shares, addr);
        let (_, idx) = vector::index_of(&pool.holders, &addr);
        vector::remove(&mut pool.holders, idx);
    }
    spec f2 {
        invariant forall i in 0..len(pool.holders), j in 0..len(pool.holders):
            pool.holders[i] == pool.holders[j] ==> i == j;

        invariant forall addr: address:
            (table::spec_contains(pool.shares, addr) <==> contains(pool.holders, addr));

        pragma verify = false;  // timeout with the default array theory
    }
}
