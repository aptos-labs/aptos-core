module 0x42::update_field_ok {
    struct R {
        x: u64,
        y: u64,
    }

    fun f(r: &mut R) {
        r.x = 1;
    }
    spec f {
        aborts_if false;
        ensures r == assign_x_1(old(r));
    }

    spec fun assign_x_1(r: R): R {
        update_field(r, x, 1)
    }
}
