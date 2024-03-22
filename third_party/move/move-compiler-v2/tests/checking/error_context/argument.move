module 0x42::m {

    fun f(_x: u64, _y: bool) {
    }

    fun g(_x: &u64, _y: &mut bool) {
    }

    fun h(_x: &address, _y: &mut address) {
    }

    fun arg_1() {
        let x = 22;
        f(1, 2);
    }

    fun arg_2() {
        let x = 1;
        let y;
        g(&1, &mut x);
        g(&true, &mut y);
    }

    fun arg_3() {
        let x = @0x1;
        h(&mut x, &true);
    }

    fun arg_4(x: &bool) {
        g(&1, x)
    }

    fun arg_5() {
        f(1);
        f(1, false, 2);
    }

}
