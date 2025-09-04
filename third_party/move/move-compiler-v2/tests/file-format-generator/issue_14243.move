// Example taken from https://github.com/velor-chain/velor-core/issues/14243
module 0xc0ffee::m {
   fun id_mut<T>(r: &mut T): &mut T {
        r
    }

    fun t0() {
        let x = &mut 0;
        let y = id_mut(x);
        *y;
        *x;
    }
}
