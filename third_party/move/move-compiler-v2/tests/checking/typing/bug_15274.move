module 0x42::test {

    struct Wrapper<T: copy> has drop, key, store, copy {
        inner: T
    }

    fun unwrap<T: copy>(self: &Wrapper<T>): T {
        self.inner
    }

    fun unwrap_non_receiver<T: copy>(self1: &Wrapper<T>): T {
        self1.inner
    }

    fun dispatch_non_receiver<T: copy>(account: address): T acquires Wrapper {
        unwrap_non_receiver(&Wrapper<T>[account])
    }

    fun dispatch<T: copy>(account: address): T acquires Wrapper {
        Wrapper<T>[account].unwrap()
    }

    fun test_vec<T>(v: vector<Wrapper<T>>): T {
        v[0].unwrap()
    }

}
