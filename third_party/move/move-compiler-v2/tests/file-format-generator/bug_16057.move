module 0xc0ffee::m {
    use std::vector;

    struct S<K, V> has store {
        keys: vector<K>,
        values: vector<V>,
        num: u64,
    }

    fun new<K: copy + drop + store, V: store>(): S<K, V> {
        S { keys: vector[], values: vector[], num: 0 }
    }

    fun add<K, V>(self: &mut S<K, V>, k: K, v: V) {
        vector::push_back(&mut self.keys, k);
        vector::push_back(&mut self.values, v);
        self.num += 1;
    }

    fun kp<K: store + copy + drop, V: store + copy>(self: &S<K, V>, i: u64): (vector<K>, u64) {
        (self.keys, i)
    }

    fun destroy<K: drop, V: drop>(self: S<K, V>) {
        let _k = vector::pop_back(&mut self.keys);
        let _v = vector::pop_back(&mut self.values);
        let S { keys, values, num: _num } = self;
        vector::destroy_empty(keys);
        vector::destroy_empty(values);
    }

    public fun test() {
        let t = new();
        t.add(1, 0);
        let n = t.num;
        t.kp(n);
        t.destroy();
    }
}
