// TODO(#14243)
//# print-bytecode --input=module
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
