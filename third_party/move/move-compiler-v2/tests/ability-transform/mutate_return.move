module 0xc0ffee::m {

    /// Return an vector of size one containing element `e`.
    public fun singleton<Element>(e: Element): vector<Element> {
        let v = vector[e];
        g(&mut v);
        v
    }

    fun g<A>(_v: &mut vector<A>) {}
}
